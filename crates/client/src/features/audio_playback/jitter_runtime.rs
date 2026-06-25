//! Runtime bridge between inbound voice frames and the jitter buffer.

use dioxus::prelude::{debug, warn};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::spawn_local;
use web_sys::{AudioContext, MessageEvent};
use web_time::{SystemTime, UNIX_EPOCH};

use super::browser_helpers::js_error_message;
use super::clock::{create_playback_clock_node, load_playback_clock_module};
use super::jitter_buffer::JitterBufferPush;
use super::provider::{AudioPlaybackHandle, PlaybackCodec, VoiceFrame};

impl AudioPlaybackHandle {
    /// Plays one encoded voice frame.
    pub(crate) fn play_voice_frame(&self, frame: VoiceFrame) {
        if frame.codec != PlaybackCodec::Opus || frame.bytes.is_empty() {
            return;
        }
        if self.is_muted() {
            return;
        }

        let sender_user_id = frame.sender_user_id.clone();
        let sequence = frame.sequence;
        let outcome = {
            let mut inner = self.inner.borrow_mut();
            if inner.muted {
                return;
            }
            inner
                .jitter_buffers
                .entry(sender_user_id.clone())
                .or_default()
                .push(frame, jitter_now_ms())
        };
        let target_delay_ms = self.inner.borrow().jitter_buffer_ms;

        match outcome {
            JitterBufferPush::Accepted { pending_frames } => {
                if pending_frames == 1 {
                    debug!(
                        %sender_user_id,
                        sequence,
                        target_delay_ms,
                        "started inbound voice jitter buffer"
                    );
                }
            }
            JitterBufferPush::Reset {
                previous_expected_sequence,
                pending_frames,
            } => {
                debug!(
                    %sender_user_id,
                    sequence,
                    previous_expected_sequence,
                    pending_frames,
                    "reset inbound voice jitter buffer after sender sequence restart"
                );
            }
            JitterBufferPush::DroppedDuplicate => {
                debug!(
                    %sender_user_id,
                    sequence,
                    "dropped duplicate inbound voice frame"
                );
                return;
            }
            JitterBufferPush::DroppedStale { expected_sequence } => {
                debug!(
                    %sender_user_id,
                    sequence,
                    expected_sequence,
                    "dropped stale inbound voice frame"
                );
                return;
            }
        }

        self.ensure_playback_clock();
    }

    /// Запускает тактовый генератор воспроизведения (один раз на сессию контекста).
    ///
    /// Слив jitter-буфера управляется тиком AudioWorklet, а не `setTimeout`, чтобы
    /// звук не деградировал, когда вкладка в фоне (там таймеры троттлятся до ~1 c).
    fn ensure_playback_clock(&self) {
        if self.inner.borrow().clock_started {
            return;
        }
        let Ok(context) = self.context() else {
            return;
        };
        self.inner.borrow_mut().clock_started = true;

        let handle = self.clone();
        spawn_local(async move {
            if let Err(error) = handle.start_playback_clock(context).await {
                warn!(
                    error = %js_error_message(error),
                    "failed to start playback clock worklet"
                );
                // Разрешаем повторную попытку на следующем фрейме.
                handle.inner.borrow_mut().clock_started = false;
            }
        });
    }

    async fn start_playback_clock(&self, context: AudioContext) -> Result<(), JsValue> {
        load_playback_clock_module(&context).await?;
        let node = create_playback_clock_node(&context)?;
        let port = node.port()?;

        let handle = self.clone();
        let closure = Closure::wrap(Box::new(move |_event: MessageEvent| {
            handle.drain_all_ready();
        }) as Box<dyn FnMut(MessageEvent)>);
        port.set_onmessage(Some(closure.as_ref().unchecked_ref()));

        let mut inner = self.inner.borrow_mut();
        inner.clock_node = Some(node);
        inner.clock_closure = Some(closure);

        debug!("started inbound voice playback clock worklet");
        Ok(())
    }

    /// Сливает готовые фреймы из буферов всех отправителей. Вызывается по тику ворклета.
    fn drain_all_ready(&self) {
        let sender_ids = {
            let inner = self.inner.borrow();
            if inner.muted {
                return;
            }
            inner.jitter_buffers.keys().cloned().collect::<Vec<_>>()
        };

        for sender_user_id in sender_ids {
            self.drain_jitter_buffer(&sender_user_id);
        }
    }

    fn drain_jitter_buffer(&self, sender_user_id: &str) {
        let (drain, target_delay_ms) = {
            let mut inner = self.inner.borrow_mut();
            if inner.muted {
                inner.jitter_buffers.remove(sender_user_id);
                return;
            }

            let target_delay_ms = inner.jitter_buffer_ms;
            let Some(buffer) = inner.jitter_buffers.get_mut(sender_user_id) else {
                return;
            };
            (
                buffer.drain_ready(jitter_now_ms(), u64::from(target_delay_ms)),
                target_delay_ms,
            )
        };

        if drain.skipped_sequences > 0 || drain.dropped_stale_frames > 0 {
            debug!(
                %sender_user_id,
                skipped_sequences = drain.skipped_sequences,
                dropped_stale_frames = drain.dropped_stale_frames,
                target_delay_ms,
                "advanced inbound voice jitter buffer after network jitter"
            );
        }

        for frame in drain.ready_frames {
            if let Err(error) = self.decode_voice_frame(frame) {
                warn!(
                    error = %js_error_message(error),
                    %sender_user_id,
                    "failed to decode jitter-buffered voice frame"
                );
            }
        }
    }
}

fn jitter_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

//! Воспроизведение коротких уведомлений через browser AudioContext.

use dioxus::prelude::{debug, warn};
use js_sys::ArrayBuffer;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{AudioBufferSourceNode, GainNode, Response};

use crate::features::audio_playback::NotificationSound;

use super::AudioPlaybackHandle;
use super::browser_helpers::{js_error_message, stop_audio_source};

#[derive(Default)]
pub(super) struct ConnectionSignalLoopState {
    source: Option<AudioBufferSourceNode>,
    gain: Option<GainNode>,
    generation: u64,
}

impl AudioPlaybackHandle {
    /// Проигрывает короткий системный звук уведомления.
    pub(crate) fn play_notification_sound(&self, sound: NotificationSound) {
        if self.is_muted() {
            return;
        }

        let handle = self.clone();
        spawn_local(async move {
            if let Err(error) = handle.play_notification_sound_async(sound).await {
                warn!(
                    sound = sound.event_name(),
                    error = %js_error_message(error),
                    "failed to play notification sound"
                );
            }
        });
    }

    /// Проигрывает звук потери полностью, затем запускает повторяющийся сигнал.
    pub(crate) fn play_connection_lost_then_loop(&self) {
        if self.is_muted() {
            return;
        }
        self.stop_connection_signal_loop();
        let generation = {
            let mut inner = self.inner.borrow_mut();
            let state = &mut inner.connection_signal_loop;
            state.generation = state.generation.saturating_add(1);
            state.generation
        };
        let handle = self.clone();
        spawn_local(async move {
            if let Err(error) = handle
                .play_connection_lost_then_loop_async(generation)
                .await
            {
                warn!(error = %js_error_message(error), "failed to play sequenced connection notification");
            }
        });
    }

    /// Останавливает повторяющийся сигнал потери соединения.
    pub(crate) fn stop_connection_signal_loop(&self) {
        let source = {
            let mut inner = self.inner.borrow_mut();
            let signal_loop = &mut inner.connection_signal_loop;
            signal_loop.generation = signal_loop.generation.saturating_add(1);
            let source = signal_loop.source.take();
            let gain = signal_loop.gain.take();
            (source, gain)
        };
        let (source, gain) = source;
        if let (Some(source), Some(gain)) = (source, gain) {
            if let Ok(context) = self.context() {
                let now = context.current_time();
                let end = now + 0.015;
                let parameter = gain.gain();
                let _ = parameter.cancel_scheduled_values(now);
                let _ = parameter.set_value_at_time(parameter.value(), now);
                let _ = parameter.linear_ramp_to_value_at_time(0.0, end);
                if let Err(error) = source.stop_with_when(end) {
                    warn!(error = %js_error_message(error), "failed to schedule connection signal fade-out");
                } else {
                    debug!(fade_ms = 15, "fading out browser connection signal loop");
                }
            } else if let Err(error) = stop_audio_source(&source) {
                warn!(error = %js_error_message(error), "failed to stop connection signal loop");
            }
        }
    }

    async fn play_notification_sound_async(&self, sound: NotificationSound) -> Result<(), JsValue> {
        let context = self.context()?;
        if let Ok(promise) = context.resume() {
            let _ = JsFuture::from(promise).await;
        }

        let window =
            web_sys::window().ok_or_else(|| JsValue::from_str("Browser window is unavailable."))?;
        let response = JsFuture::from(window.fetch_with_str(sound.asset_path()))
            .await?
            .dyn_into::<Response>()?;
        if !response.ok() {
            return Err(JsValue::from_str(&format!(
                "Notification sound fetch failed with HTTP status {}.",
                response.status()
            )));
        }

        let bytes = JsFuture::from(response.array_buffer()?)
            .await?
            .dyn_into::<ArrayBuffer>()?;
        let buffer = JsFuture::from(context.decode_audio_data(&bytes)?).await?;
        let buffer = buffer.dyn_into::<web_sys::AudioBuffer>()?;
        let source = context.create_buffer_source()?;
        let gain = context.create_gain()?;
        let output_gain = self.inner.borrow().output_gain;

        gain.gain()
            .set_value(output_gain as f32 * sound.volume_multiplier());
        source.set_buffer(Some(&buffer));
        source.connect_with_audio_node(&gain)?;
        gain.connect_with_audio_node(&context.destination())?;
        source.start()?;

        debug!(sound = sound.event_name(), "played notification sound");
        Ok(())
    }

    async fn play_connection_lost_then_loop_async(&self, generation: u64) -> Result<(), JsValue> {
        let context = self.context()?;
        if let Ok(promise) = context.resume() {
            let _ = JsFuture::from(promise).await;
        }
        let lost = fetch_notification_buffer(&context, NotificationSound::ConnectionLost).await?;
        if !self.connection_generation_is_current(generation) {
            return Ok(());
        }
        let lost_source = context.create_buffer_source()?;
        let lost_gain = context.create_gain()?;
        lost_source.set_buffer(Some(&lost));
        lost_gain.gain().set_value(
            self.inner.borrow().output_gain as f32
                * NotificationSound::ConnectionLost.volume_multiplier(),
        );
        lost_source.connect_with_audio_node(&lost_gain)?;
        lost_gain.connect_with_audio_node(&context.destination())?;
        let lost_end = context.current_time() + lost.duration();
        lost_source.start()?;
        let loop_buffer =
            fetch_notification_buffer(&context, NotificationSound::ConnectionSignalLoop).await?;
        if !self.connection_generation_is_current(generation) {
            return Ok(());
        }
        let source = context.create_buffer_source()?;
        let gain = context.create_gain()?;
        source.set_buffer(Some(&loop_buffer));
        source.set_loop(true);
        gain.gain()
            .set_value(self.inner.borrow().output_gain as f32);
        source.connect_with_audio_node(&gain)?;
        gain.connect_with_audio_node(&context.destination())?;
        {
            let mut inner = self.inner.borrow_mut();
            inner.connection_signal_loop.source = Some(source.clone());
            inner.connection_signal_loop.gain = Some(gain);
        }
        let loop_start_at = connection_loop_start_time(context.current_time(), lost_end);
        source.start_with_when(loop_start_at)?;
        debug!(
            lost_duration_seconds = lost.duration(),
            "scheduled browser signal loop after connection lost sample"
        );
        Ok(())
    }

    fn connection_generation_is_current(&self, generation: u64) -> bool {
        let inner = self.inner.borrow();
        inner.connection_signal_loop.generation == generation && !inner.muted
    }
}

fn connection_loop_start_time(current_time: f64, lost_end_time: f64) -> f64 {
    current_time.max(lost_end_time)
}

async fn fetch_notification_buffer(
    context: &web_sys::AudioContext,
    sound: NotificationSound,
) -> Result<web_sys::AudioBuffer, JsValue> {
    let window =
        web_sys::window().ok_or_else(|| JsValue::from_str("Browser window is unavailable."))?;
    let response = JsFuture::from(window.fetch_with_str(sound.asset_path()))
        .await?
        .dyn_into::<Response>()?;
    if !response.ok() {
        return Err(JsValue::from_str(&format!(
            "Notification sound fetch failed with HTTP status {}.",
            response.status()
        )));
    }
    let bytes = JsFuture::from(response.array_buffer()?)
        .await?
        .dyn_into::<ArrayBuffer>()?;
    JsFuture::from(context.decode_audio_data(&bytes)?)
        .await?
        .dyn_into::<web_sys::AudioBuffer>()
}

#[cfg(test)]
mod tests {
    use super::connection_loop_start_time;

    #[test]
    fn loop_starts_after_lost_sample_finishes() {
        assert_eq!(connection_loop_start_time(10.0, 12.5), 12.5);
    }

    #[test]
    fn loop_starts_immediately_when_lost_sample_has_already_finished() {
        assert_eq!(connection_loop_start_time(13.0, 12.5), 13.0);
    }
}

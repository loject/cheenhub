//! Воспроизведение коротких уведомлений через browser AudioContext.

use dioxus::prelude::{debug, warn};
use js_sys::ArrayBuffer;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{AudioBufferSourceNode, Response};

use crate::features::audio_playback::NotificationSound;

use super::AudioPlaybackHandle;
use super::browser_helpers::{js_error_message, stop_audio_source};

#[derive(Default)]
pub(super) struct ConnectionSignalLoopState {
    source: Option<AudioBufferSourceNode>,
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

    /// Запускает повторяющийся сигнал потери соединения для активного голосового чата.
    pub(crate) fn start_connection_signal_loop(&self) {
        if self.is_muted() {
            return;
        }
        self.stop_connection_signal_loop();
        let generation = {
            let mut inner = self.inner.borrow_mut();
            let signal_loop = &mut inner.connection_signal_loop;
            signal_loop.generation = signal_loop.generation.saturating_add(1);
            signal_loop.generation
        };
        let handle = self.clone();
        spawn_local(async move {
            if let Err(error) = handle.start_connection_signal_loop_async(generation).await {
                warn!(
                    error = %js_error_message(error),
                    "failed to start connection signal loop"
                );
            }
        });
    }

    /// Останавливает повторяющийся сигнал потери соединения.
    pub(crate) fn stop_connection_signal_loop(&self) {
        let source = {
            let mut inner = self.inner.borrow_mut();
            let signal_loop = &mut inner.connection_signal_loop;
            signal_loop.generation = signal_loop.generation.saturating_add(1);
            signal_loop.source.take()
        };
        if let Some(source) = source
            && let Err(error) = stop_audio_source(&source)
        {
            warn!(
                error = %js_error_message(error),
                "failed to stop connection signal loop"
            );
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

        gain.gain().set_value(output_gain as f32);
        source.set_buffer(Some(&buffer));
        source.connect_with_audio_node(&gain)?;
        gain.connect_with_audio_node(&context.destination())?;
        source.start()?;

        debug!(sound = sound.event_name(), "played notification sound");
        Ok(())
    }

    async fn start_connection_signal_loop_async(&self, generation: u64) -> Result<(), JsValue> {
        let context = self.context()?;
        if let Ok(promise) = context.resume() {
            let _ = JsFuture::from(promise).await;
        }
        let window =
            web_sys::window().ok_or_else(|| JsValue::from_str("Browser window is unavailable."))?;
        let response = JsFuture::from(
            window.fetch_with_str(NotificationSound::ConnectionSignalLoop.asset_path()),
        )
        .await?
        .dyn_into::<Response>()?;
        if !response.ok() {
            return Err(JsValue::from_str(&format!(
                "Connection signal loop fetch failed with HTTP status {}.",
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
        source.set_buffer(Some(&buffer));
        source.set_loop(true);
        gain.gain().set_value(output_gain as f32);
        source.connect_with_audio_node(&gain)?;
        gain.connect_with_audio_node(&context.destination())?;

        let should_start = {
            let mut inner = self.inner.borrow_mut();
            if inner.connection_signal_loop.generation != generation || inner.muted {
                false
            } else {
                inner.connection_signal_loop.source = Some(source.clone());
                true
            }
        };
        if should_start {
            source.start()?;
            debug!("started browser connection signal loop");
        }
        Ok(())
    }
}

//! Microphone context provider component.

use std::rc::Rc;

use dioxus::prelude::*;

use super::backend::{
    MicrophoneConfig, MicrophoneFrameCallback, MicrophoneSession, MicrophoneStatus,
    MicrophoneUplinkConfig,
};
use super::native::default_backend;
use super::provider::{ActiveCapture, MicrophoneHandle};
use super::provider_runtime::{MicrophoneRuntime, default_level, run_microphone_runtime};
use super::storage;
use crate::features::toast::ToastHandle;

/// Provides microphone capture state to authenticated app components.
#[component]
pub(crate) fn MicrophoneProvider(children: Element) -> Element {
    let _ptt_availability = use_resource(super::push_to_talk::availability);
    use_drop(super::push_to_talk::platform::shutdown);
    let status = use_signal(|| MicrophoneStatus::Idle);
    let voice_enabled = use_signal(storage::load_voice_enabled);
    let level = use_signal(default_level);
    let level_active = use_signal(|| false);
    let session = use_signal(|| None::<Rc<dyn MicrophoneSession>>);
    let generation = use_signal(|| 0);
    let stored_input_device = storage::load_input_device();
    let selected_input_device_id = use_signal({
        let stored_input_device = stored_input_device.clone();
        move || {
            stored_input_device
                .as_ref()
                .map(|device| device.device_id.clone())
        }
    });
    let selected_input_device_label =
        use_signal(move || stored_input_device.and_then(|device| device.label));
    let input_volume_percent = use_signal(storage::load_input_volume_percent);
    let activation_mode = use_signal(storage::load_activation_mode);
    let push_to_talk_key = use_signal(storage::load_push_to_talk_key);
    let mut push_to_talk_update = use_signal(|| None);
    use_future(move || async move {
        let mut key_signal = push_to_talk_key;
        super::push_to_talk::observe_updates(move |update| {
            if let Some(key) = update.binding {
                storage::save_push_to_talk_key(key);
                key_signal.set(key);
            }
            push_to_talk_update.set(Some(update));
        })
        .await;
    });
    let recording_generation =
        use_hook(|| std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)));
    let vad_threshold_percent = use_signal(storage::load_vad_threshold_percent);
    let neural_vad_threshold_percent = use_signal(storage::load_neural_vad_threshold_percent);
    let initial_mode = *activation_mode.peek();
    let initial_threshold = *vad_threshold_percent.peek();
    let activation_runtime = use_hook(move || {
        let config = MicrophoneConfig::default();
        std::sync::Arc::new(std::sync::Mutex::new(super::backend::ActivationConfig {
            mode: initial_mode,
            threshold: super::provider_runtime::threshold_from_percent(initial_threshold),
            neural_threshold: *neural_vad_threshold_percent.peek() as f32 / 100.0,
            activation_delay_us: config.vad_activation_delay_us,
            release_delay_us: config.vad_release_delay_us,
        }))
    });
    let stored_denoise = storage::load_denoise_mode();
    let denoise_mode = use_signal(move || stored_denoise);
    let denoise_failed = use_signal(|| false);
    let denoise_pending = use_signal(|| false);
    let denoise_runtime = use_hook(move || {
        std::sync::Arc::new(super::denoise::RuntimeSwitch::new(
            stored_denoise == super::denoise::DenoiseMode::Rnnoise,
        ))
    });
    let active_capture = use_signal(|| ActiveCapture::None);
    let active_on_frame = use_signal(|| None::<MicrophoneFrameCallback>);
    let active_uplink = use_signal(|| None::<MicrophoneUplinkConfig>);
    let target_bitrate_bps = use_signal(|| MicrophoneConfig::default().bitrate_bps);
    let backend = default_backend();
    let toast = use_context::<ToastHandle>();
    let denoise_cancel = use_hook(|| Rc::new(std::cell::RefCell::new(None)));
    let runtime = MicrophoneRuntime {
        denoise_cancel,
        denoise_timeout: super::provider_runtime::denoise_timeout,
        backend,
        status,
        level,
        level_active,
        session,
        generation,
        selected_input_device_id,
        input_volume_percent,
        activation_mode,
        activation_runtime: activation_runtime.clone(),
        push_to_talk_key,
        recording_generation: recording_generation.clone(),
        vad_threshold_percent,
        neural_vad_threshold_percent,
        denoise_mode,
        denoise_pending,
        denoise_failed,
        denoise_runtime: denoise_runtime.clone(),
        toast,
        active_capture,
        active_on_frame,
        active_uplink,
        target_bitrate_bps,
    };
    let commands = use_coroutine(move |receiver| run_microphone_runtime(receiver, runtime.clone()));
    let handle = MicrophoneHandle {
        voice_enabled,
        status,
        level,
        level_active,
        generation,
        commands: commands.tx(),
        selected_input_device_id,
        selected_input_device_label,
        input_volume_percent,
        activation_mode,
        activation_runtime: activation_runtime.clone(),
        push_to_talk_key,
        push_to_talk_update,
        recording_generation: recording_generation.clone(),
        vad_threshold_percent,
        neural_vad_threshold_percent,
        denoise_mode,
        denoise_pending,
        denoise_failed,
        denoise_runtime: denoise_runtime.clone(),
        active_capture,
        active_on_frame,
        active_uplink,
        target_bitrate_bps,
    };
    use_context_provider(move || handle.clone());

    rsx! {
        {children}
    }
}

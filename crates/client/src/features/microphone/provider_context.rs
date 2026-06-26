//! Microphone context provider component.

use std::rc::Rc;

use dioxus::prelude::*;

use super::backend::{MicrophoneFrameCallback, MicrophoneSession, MicrophoneStatus};
use super::native::default_backend;
use super::provider::{ActiveCapture, MicrophoneHandle};
use super::provider_runtime::default_level;
use super::storage;

/// Provides microphone capture state to authenticated app components.
#[component]
pub(crate) fn MicrophoneProvider(children: Element) -> Element {
    let status = use_signal(|| MicrophoneStatus::Idle);
    let level = use_signal(default_level);
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
    let vad_threshold_percent = use_signal(storage::load_vad_threshold_percent);
    let active_capture = use_signal(|| ActiveCapture::None);
    let active_on_frame = use_signal(|| None::<MicrophoneFrameCallback>);
    let backend = default_backend();
    let handle = MicrophoneHandle {
        status,
        level,
        session,
        generation,
        backend,
        selected_input_device_id,
        selected_input_device_label,
        input_volume_percent,
        activation_mode,
        vad_threshold_percent,
        active_capture,
        active_on_frame,
    };
    use_context_provider(move || handle.clone());

    rsx! {
        {children}
    }
}

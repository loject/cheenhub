//! Voice connection context provider.

use std::rc::Rc;

use cheenhub_contracts::rest::AuthUser;
use dioxus::prelude::*;
use futures_util::StreamExt;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::features::microphone::MicrophoneHandle;
use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle};

use super::realtime;
use super::state::{VoiceConnectionHandle, VoiceConnectionState};

/// Provides voice connection state to authenticated app components.
#[component]
pub(crate) fn VoiceConnectionProvider(current_user: AuthUser, children: Element) -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let microphone = use_context::<MicrophoneHandle>();
    let state = use_signal(|| VoiceConnectionState::Disconnected);
    let mut microphone_target_room = use_signal(|| None::<String>);
    let handle = VoiceConnectionHandle::new(state, realtime.clone(), current_user);
    let context_handle = handle.clone();
    use_context_provider(move || context_handle.clone());

    let snapshot_realtime = realtime.clone();
    use_hook(move || {
        spawn(async move {
            let mut snapshots = realtime::subscribe_voice_chat(&snapshot_realtime);
            while let Some(snapshot) = snapshots.next().await {
                handle.apply_snapshot(snapshot);
            }
        })
    });
    let status_realtime = realtime.clone();
    use_hook(move || {
        spawn(async move {
            let mut statuses = status_realtime.subscribe_connection_status();
            while let Some(status) = statuses.next().await {
                if status == RealtimeConnectionStatus::Disconnected {
                    let mut state = state;
                    state.set(VoiceConnectionState::Disconnected);
                }
            }
        })
    });
    use_effect(move || match state() {
        VoiceConnectionState::Connected { target, .. } => {
            if microphone_target_room().as_deref() == Some(target.room_id.as_str()) {
                return;
            }
            microphone_target_room.set(Some(target.room_id.clone()));
            restart_microphone_for_target(
                microphone.clone(),
                realtime.clone(),
                target.server_id,
                target.room_id,
            );
        }
        VoiceConnectionState::Disconnected
        | VoiceConnectionState::Connecting { .. }
        | VoiceConnectionState::Disconnecting { .. }
        | VoiceConnectionState::Error { .. } => {
            if microphone_target_room().is_some() {
                microphone_target_room.set(None);
                microphone.stop();
            }
        }
    });

    rsx! {
        {children}
    }
}

fn restart_microphone_for_target(
    microphone: MicrophoneHandle,
    realtime_handle: RealtimeHandle,
    server_id: String,
    room_id: String,
) {
    microphone.restart(Rc::new(move |frame| {
        let frame_realtime = realtime_handle.clone();
        let frame_server_id = server_id.clone();
        let frame_room_id = room_id.clone();
        spawn_local(async move {
            if let Err(error) =
                realtime::send_voice_frame(&frame_realtime, &frame_server_id, &frame_room_id, frame)
                    .await
            {
                web_sys::console::warn_1(&JsValue::from_str(&format!(
                    "failed to send encoded voice frame: {error}"
                )));
            }
        });
    }));
}

//! Voice connection context provider.

use std::rc::Rc;

use dioxus::prelude::*;
use futures_util::StreamExt;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::features::app::current_user::CurrentUserContext;
use crate::features::audio_playback::{AudioPlaybackHandle, PlaybackCodec, VoiceFrame};
use crate::features::microphone::{MicrophoneHandle, MicrophoneStatus};
use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle};

use super::realtime;
use super::state::{VoiceConnectionHandle, VoiceConnectionState};

/// Provides voice connection state to authenticated app components.
#[component]
pub(crate) fn VoiceConnectionProvider(children: Element) -> Element {
    let current_user = use_context::<CurrentUserContext>().require_user();
    let realtime = use_context::<RealtimeHandle>();
    let microphone = use_context::<MicrophoneHandle>();
    let playback = use_context::<AudioPlaybackHandle>();
    let state = use_signal(|| VoiceConnectionState::Disconnected);
    let speaking_users = use_signal(Vec::new);
    let mut microphone_target_room = use_signal(|| None::<String>);
    let mut mic_paused_by_mute = use_signal(|| false);
    let handle = VoiceConnectionHandle::new(
        state,
        speaking_users,
        realtime.clone(),
        current_user.clone(),
    );
    let context_handle = handle.clone();
    use_context_provider(move || context_handle.clone());

    let snapshot_realtime = realtime.clone();
    let snapshot_handle = handle.clone();
    use_hook(move || {
        spawn(async move {
            let mut snapshots = realtime::subscribe_voice_chat(&snapshot_realtime);
            while let Some(snapshot) = snapshots.next().await {
                snapshot_handle.apply_snapshot(snapshot);
            }
        })
    });
    let datagram_realtime = realtime.clone();
    let datagram_playback = playback.clone();
    let datagram_current_user_id = current_user.id.clone();
    let datagram_handle = handle.clone();
    use_hook(move || {
        spawn(async move {
            let mut frames = realtime::subscribe_voice_frames(&datagram_realtime);
            while let Some(frame) = frames.next().await {
                let current = state();
                let Some(target) = current.active_target() else {
                    continue;
                };
                if frame.room_id != target.room_id
                    || frame.sender_user_id == datagram_current_user_id
                {
                    continue;
                }
                datagram_handle.mark_user_speaking(frame.sender_user_id.clone());
                datagram_playback.play_voice_frame(VoiceFrame {
                    sender_user_id: frame.sender_user_id,
                    sequence: frame.sequence,
                    timestamp_us: frame.timestamp_us,
                    duration_us: frame.duration_us,
                    codec: PlaybackCodec::Opus,
                    bytes: frame.bytes,
                });
            }
        })
    });
    let status_realtime = realtime.clone();
    let status_playback = playback.clone();
    let status_handle = handle.clone();
    use_hook(move || {
        spawn(async move {
            let mut statuses = status_realtime.subscribe_connection_status();
            while let Some(status) = statuses.next().await {
                if status == RealtimeConnectionStatus::Disconnected {
                    let mut state = state;
                    state.set(VoiceConnectionState::Disconnected);
                    status_handle.clear_speaking_users();
                    status_playback.stop_all();
                }
            }
        })
    });
    let effect_handle = handle.clone();
    use_effect(move || match state() {
        VoiceConnectionState::Connected { target, .. } => {
            if playback.is_muted() {
                if !mic_paused_by_mute()
                    && matches!(
                        microphone.status_untracked(),
                        MicrophoneStatus::Live | MicrophoneStatus::Starting
                    )
                {
                    mic_paused_by_mute.set(true);
                }
                microphone.stop();
                return;
            }

            playback.resume();
            let paused_by_mute = mic_paused_by_mute();
            if paused_by_mute {
                mic_paused_by_mute.set(false);
            }
            if microphone_target_room().as_deref() == Some(target.room_id.as_str()) {
                if paused_by_mute {
                    restart_microphone_for_target(
                        microphone.clone(),
                        realtime.clone(),
                        target.server_id,
                        target.room_id,
                    );
                }
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
                mic_paused_by_mute.set(false);
                microphone.stop();
                effect_handle.clear_speaking_users();
                playback.stop_all();
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

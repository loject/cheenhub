//! Провайдер контекста голосового соединения.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use futures_util::StreamExt;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::features::app::current_user::CurrentUserContext;
use crate::features::audio_playback::{AudioPlaybackHandle, PlaybackCodec, VoiceFrame};
use crate::features::camera::{CameraHandle, CameraStatus};
use crate::features::microphone::{MicrophoneHandle, MicrophoneStatus};
use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle};
use crate::features::screen_share::{ScreenShareHandle, ScreenShareStatus};

use super::kicked_modal::KickedFromVoiceModal;
use super::realtime;
use super::state::{VoiceConnectionHandle, VoiceConnectionState};
use super::video_streams::{ParticipantVideoHandle, ParticipantVideoSource};

/// Предоставляет состояние голосового соединения аутентифицированным компонентам приложения.
/// TODO: review, выглядит сложно и как куча бойлерплейта
#[component]
pub(crate) fn VoiceConnectionProvider(children: Element) -> Element {
    let current_user = use_context::<CurrentUserContext>().require_user();
    let realtime = use_context::<RealtimeHandle>();
    let microphone = use_context::<MicrophoneHandle>();
    let camera = use_context::<CameraHandle>();
    let screen_share = use_context::<ScreenShareHandle>();
    let playback = use_context::<AudioPlaybackHandle>();
    let state = use_signal(|| VoiceConnectionState::Disconnected);
    let kicked_from_room = use_signal(|| None::<String>);
    let speaking_users = use_signal(Vec::new);
    let room_snapshots = use_signal(Vec::new);
    let speaking_generations = use_hook(|| Rc::new(RefCell::new(HashMap::<String, u64>::new())));
    let participant_video_streams = use_signal(Vec::new);
    let participant_video_subscribers = use_hook(|| Rc::new(RefCell::new(HashMap::new())));
    let participant_video_generations = use_hook(|| Rc::new(RefCell::new(HashMap::new())));
    let participant_video = ParticipantVideoHandle::new(
        participant_video_streams,
        participant_video_subscribers,
        participant_video_generations,
    );
    let participant_video_context = participant_video.clone();
    use_context_provider(move || participant_video_context.clone());
    let mut microphone_target_room = use_signal(|| None::<String>);
    let mut camera_target_room = use_signal(|| None::<String>);
    let mut screen_share_target_room = use_signal(|| None::<String>);
    let mut mic_paused_by_mute = use_signal(|| false);
    let handle = VoiceConnectionHandle::new(
        state,
        kicked_from_room,
        speaking_users,
        room_snapshots,
        speaking_generations,
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
    let screen_datagram_realtime = realtime.clone();
    let screen_datagram_current_user_id = current_user.id.clone();
    let screen_datagram_video = participant_video.clone();
    use_hook(move || {
        spawn(async move {
            let mut frames = realtime::subscribe_screen_frames(&screen_datagram_realtime);
            while let Some(frame) = frames.next().await {
                let current = state();
                let Some(target) = current.active_target() else {
                    continue;
                };
                if frame.room_id != target.room_id
                    || frame.sender_user_id == screen_datagram_current_user_id
                {
                    continue;
                }
                screen_datagram_video
                    .publish_frame(ParticipantVideoSource::ScreenShare, frame.into());
            }
        })
    });
    let camera_datagram_realtime = realtime.clone();
    let camera_datagram_current_user_id = current_user.id.clone();
    let camera_datagram_video = participant_video.clone();
    use_hook(move || {
        spawn(async move {
            let mut frames = realtime::subscribe_camera_frames(&camera_datagram_realtime);
            while let Some(frame) = frames.next().await {
                let current = state();
                let Some(target) = current.active_target() else {
                    continue;
                };
                if frame.room_id != target.room_id
                    || frame.sender_user_id == camera_datagram_current_user_id
                {
                    continue;
                }
                camera_datagram_video.publish_frame(ParticipantVideoSource::Camera, frame.into());
            }
        })
    });
    let status_realtime = realtime.clone();
    let status_playback = playback.clone();
    let status_handle = handle.clone();
    let status_participant_video = participant_video.clone();
    use_hook(move || {
        spawn(async move {
            let mut statuses = status_realtime.subscribe_connection_status();
            while let Some(status) = statuses.next().await {
                if matches!(status, RealtimeConnectionStatus::Disconnected) {
                    let mut state = state;
                    state.set(VoiceConnectionState::Disconnected);
                    status_handle.clear_speaking_users();
                    status_participant_video.clear();
                    status_playback.stop_all();
                }
            }
        })
    });
    let effect_handle = handle.clone();
    use_effect(move || match state() {
        VoiceConnectionState::Connected { target, .. } => {
            reconcile_camera_target(
                camera.clone(),
                &mut camera_target_room,
                target.room_id.as_str(),
            );
            reconcile_screen_share_target(
                screen_share.clone(),
                &mut screen_share_target_room,
                target.room_id.as_str(),
            );
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
            let had_camera_target = camera_target_room().is_some();
            let had_screen_share_target = screen_share_target_room().is_some();
            let had_microphone_target = microphone_target_room().is_some();
            let was_paused_by_mute = mic_paused_by_mute();

            if had_camera_target {
                camera_target_room.set(None);
                camera.stop();
            }
            if had_screen_share_target {
                screen_share_target_room.set(None);
                screen_share.stop();
            }
            participant_video.clear();
            if had_microphone_target {
                microphone_target_room.set(None);
                microphone.stop();
            }
            if was_paused_by_mute {
                mic_paused_by_mute.set(false);
            }
            effect_handle.clear_speaking_users();
            playback.stop_all();

            if had_camera_target
                || had_screen_share_target
                || had_microphone_target
                || was_paused_by_mute
            {
                info!(
                    microphone = had_microphone_target,
                    microphone_paused_by_mute = was_paused_by_mute,
                    camera = had_camera_target,
                    screen_share = had_screen_share_target,
                    "released local voice media resources"
                );
            }
        }
    });

    rsx! {
        {children}
        if let Some(room_name) = kicked_from_room() {
            KickedFromVoiceModal {
                room_name,
                on_close: move |_| handle.dismiss_kick_notification(),
            }
        }
    }
}

fn reconcile_camera_target(
    camera: CameraHandle,
    target_room_signal: &mut Signal<Option<String>>,
    active_room_id: &str,
) {
    if !matches!(camera.status(), CameraStatus::Live | CameraStatus::Starting) {
        if target_room_signal().is_some() {
            target_room_signal.set(None);
        }
        return;
    }

    match target_room_signal().as_deref() {
        Some(room_id) if room_id == active_room_id => {}
        Some(_) => {
            info!("stopping camera capture after active voice room changed");
            target_room_signal.set(None);
            camera.stop();
        }
        None => target_room_signal.set(Some(active_room_id.to_owned())),
    }
}

fn reconcile_screen_share_target(
    screen_share: ScreenShareHandle,
    target_room_signal: &mut Signal<Option<String>>,
    active_room_id: &str,
) {
    if !matches!(
        screen_share.status(),
        ScreenShareStatus::Live | ScreenShareStatus::Starting
    ) {
        if target_room_signal().is_some() {
            target_room_signal.set(None);
        }
        return;
    }

    match target_room_signal().as_deref() {
        Some(room_id) if room_id == active_room_id => {}
        Some(_) => {
            info!("stopping screen sharing capture after active voice room changed");
            target_room_signal.set(None);
            screen_share.stop();
        }
        None => target_room_signal.set(Some(active_room_id.to_owned())),
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

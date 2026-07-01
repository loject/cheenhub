//! Провайдер контекста голосового соединения.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::app::current_user::CurrentUserContext;
use crate::features::audio_playback::{
    AudioPlaybackHandle, NotificationSound, PlaybackCodec, VoiceFrame,
};
use crate::features::camera::{CameraHandle, CameraStatus};
use crate::features::microphone::{MicrophoneHandle, MicrophoneStatus};
use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle};
use crate::features::screen_share::{ScreenShareHandle, ScreenShareStatus};

use super::kicked_modal::KickedFromVoiceModal;
use super::local_video::{
    LocalVideoRuntime, LocalVideoTarget, participant_source_from_contract, reconcile_camera_target,
    reconcile_screen_share_target, release_local_video_target,
};
use super::notification_sounds::{
    ConnectionNotificationSoundState, ToggleNotificationSoundState, VoiceNotificationSoundState,
};
use super::realtime;
use super::state::{VoiceConnectionHandle, VoiceConnectionState};
use super::video_streams::{ParticipantVideoHandle, ParticipantVideoSource};
use super::voice_frame_sender;

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
    let participant_video_blocked_streams = use_hook(|| Rc::new(RefCell::new(HashSet::new())));
    let participant_video = ParticipantVideoHandle::new(
        participant_video_streams,
        participant_video_subscribers,
        participant_video_generations,
        participant_video_blocked_streams,
    );
    let participant_video_context = participant_video.clone();
    use_context_provider(move || participant_video_context.clone());
    let mut microphone_target_room = use_signal(|| None::<String>);
    let mut camera_target_room = use_signal(|| None::<LocalVideoTarget>);
    let mut screen_share_target_room = use_signal(|| None::<LocalVideoTarget>);
    let mut mic_paused_by_mute = use_signal(|| false);
    let voice_notification_sounds =
        use_hook(|| Rc::new(RefCell::new(VoiceNotificationSoundState::default())));
    let camera_notification_sounds =
        use_hook(|| Rc::new(RefCell::new(ToggleNotificationSoundState::default())));
    let screen_share_notification_sounds =
        use_hook(|| Rc::new(RefCell::new(ToggleNotificationSoundState::default())));
    let connection_notification_sounds =
        use_hook(|| Rc::new(RefCell::new(ConnectionNotificationSoundState::default())));
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
    let video_ended_realtime = realtime.clone();
    let video_ended_current_user_id = current_user.id.clone();
    let video_ended_participant_video = participant_video.clone();
    use_hook(move || {
        spawn(async move {
            let mut events = realtime::subscribe_video_stream_ended(&video_ended_realtime);
            while let Some(event) = events.next().await {
                let current = state();
                let Some(target) = current.active_target() else {
                    continue;
                };
                if event.server_id != target.server_id
                    || event.room_id != target.room_id
                    || event.sender_user_id == video_ended_current_user_id
                {
                    continue;
                }
                video_ended_participant_video.release_stream(
                    participant_source_from_contract(event.source),
                    &event.sender_user_id,
                );
            }
        })
    });
    let status_realtime = realtime.clone();
    let status_playback = playback.clone();
    let status_handle = handle.clone();
    let status_participant_video = participant_video.clone();
    let status_connection_sounds = connection_notification_sounds.clone();
    use_hook(move || {
        spawn(async move {
            let mut statuses = status_realtime.subscribe_connection_status();
            while let Some(status) = statuses.next().await {
                let connected = matches!(status, RealtimeConnectionStatus::Connected(_));
                if matches!(status, RealtimeConnectionStatus::Disconnected) {
                    let mut state = state;
                    state.set(VoiceConnectionState::Disconnected);
                    status_handle.clear_speaking_users();
                    status_participant_video.clear();
                    status_playback.stop_all();
                }
                status_connection_sounds
                    .borrow_mut()
                    .record(connected, &status_playback);
            }
        })
    });
    let camera_sound_playback = playback.clone();
    let camera_sound_handle = camera.clone();
    let camera_sound_state = camera_notification_sounds.clone();
    use_effect(move || {
        camera_sound_state.borrow_mut().record(
            matches!(camera_sound_handle.status(), CameraStatus::Live),
            NotificationSound::CameraEnabled,
            NotificationSound::CameraDisabled,
            &camera_sound_playback,
        );
    });
    let screen_share_sound_playback = playback.clone();
    let screen_share_sound_handle = screen_share.clone();
    let screen_share_sound_state = screen_share_notification_sounds.clone();
    use_effect(move || {
        screen_share_sound_state.borrow_mut().record(
            matches!(screen_share_sound_handle.status(), ScreenShareStatus::Live),
            NotificationSound::ScreenShareEnabled,
            NotificationSound::ScreenShareDisabled,
            &screen_share_sound_playback,
        );
    });
    let effect_handle = handle.clone();
    let effect_current_user_id = current_user.id.clone();
    let effect_voice_sounds = voice_notification_sounds.clone();
    use_effect(move || match state() {
        VoiceConnectionState::Connected {
            target,
            participants,
        } => {
            effect_voice_sounds.borrow_mut().record_connected(
                &target,
                &participants,
                &effect_current_user_id,
                &playback,
            );
            let local_video_runtime = LocalVideoRuntime {
                realtime: realtime.clone(),
                participant_video: participant_video.clone(),
                current_user_id: effect_current_user_id.clone(),
            };
            reconcile_camera_target(
                camera.clone(),
                local_video_runtime.clone(),
                &mut camera_target_room,
                &target,
            );
            reconcile_screen_share_target(
                screen_share.clone(),
                local_video_runtime,
                &mut screen_share_target_room,
                &target,
            );
            if playback.is_muted() {
                info!(
                    target_kind = ?target.kind,
                    server_id = %target.server_id,
                    room_id = %target.room_id,
                    "voice media start paused while output is muted"
                );
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
                        target.server_id.clone(),
                        target.room_id.clone(),
                    );
                }
                info!(
                    target_kind = ?target.kind,
                    server_id = %target.server_id,
                    room_id = %target.room_id,
                    microphone_paused_by_mute = paused_by_mute,
                    "voice media already targets active room"
                );
                return;
            }
            microphone_target_room.set(Some(target.room_id.clone()));
            info!(
                target_kind = ?target.kind,
                server_id = %target.server_id,
                room_id = %target.room_id,
                "starting voice media for active room"
            );
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
            let previous_camera_target = camera_target_room();
            let previous_screen_share_target = screen_share_target_room();
            let had_camera_target = previous_camera_target.is_some();
            let had_screen_share_target = previous_screen_share_target.is_some();
            let had_microphone_target = microphone_target_room().is_some();
            let was_paused_by_mute = mic_paused_by_mute();

            if let Some(target) = previous_camera_target {
                let local_video_runtime = LocalVideoRuntime {
                    realtime: realtime.clone(),
                    participant_video: participant_video.clone(),
                    current_user_id: effect_current_user_id.clone(),
                };
                release_local_video_target(
                    local_video_runtime,
                    target,
                    ParticipantVideoSource::Camera,
                );
                camera_target_room.set(None);
                camera.stop();
            }
            if let Some(target) = previous_screen_share_target {
                let local_video_runtime = LocalVideoRuntime {
                    realtime: realtime.clone(),
                    participant_video: participant_video.clone(),
                    current_user_id: effect_current_user_id.clone(),
                };
                release_local_video_target(
                    local_video_runtime,
                    target,
                    ParticipantVideoSource::ScreenShare,
                );
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
            if effect_voice_sounds.borrow().is_current_user_connected() {
                playback.stop_all();
            }
            effect_voice_sounds.borrow_mut().record_inactive(&playback);

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

fn restart_microphone_for_target(
    microphone: MicrophoneHandle,
    realtime_handle: RealtimeHandle,
    server_id: String,
    room_id: String,
) {
    microphone.restart(voice_frame_sender::voice_frame_sender_callback(
        realtime_handle,
        server_id,
        room_id,
    ));
}

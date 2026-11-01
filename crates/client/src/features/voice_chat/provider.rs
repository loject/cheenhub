//! Провайдер контекста голосового соединения.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::app::current_user::CurrentUserContext;
use crate::features::audio_playback::{AudioPlaybackHandle, NotificationSound};
use crate::features::camera::{CameraHandle, CameraStatus};
use crate::features::microphone::{MicrophoneHandle, MicrophoneStatus};
use crate::features::network::NetworkQualityHandle;
use crate::features::realtime::RealtimeHandle;
use crate::features::screen_share::{ScreenShareHandle, ScreenShareStatus};
use crate::features::system_tray::SystemTrayHandle;

use super::direct_call_provider::DirectCallProvider;
use super::kicked_modal::KickedFromVoiceModal;
use super::local_video::{
    LocalVideoRuntime, LocalVideoTarget, reconcile_camera_target, reconcile_screen_share_target,
    release_local_video_target,
};
use super::microphone_uplink;
use super::network_quality::use_voice_network_quality_runtime;
use super::notification_sounds::{
    ConnectionNotificationSoundState, ToggleNotificationSoundState, VoiceNotificationSoundState,
};
use super::realtime;
use super::room_invite_provider::RoomInviteProvider;
use super::state::{
    VoiceConnectionHandle, VoiceConnectionParts, VoiceConnectionState, VoiceMediaSession,
    VoiceRoomTarget, VoiceRoomTargetKind,
};
use super::video_streams::{ParticipantVideoHandle, ParticipantVideoSource};
use super::voice_call_platform;
use super::voice_recovery;
use super::voice_session_dialogs::VoiceSessionDialogs;

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
    let system_tray = use_context::<SystemTrayHandle>();
    use_drop({
        let system_tray = system_tray.clone();
        move || system_tray.reset_voice_indicator()
    });
    let network_quality = use_context::<NetworkQualityHandle>();
    let state = use_signal(|| VoiceConnectionState::Disconnected);
    let mut platform_call_active = use_signal(|| false);
    let voice_audio_focused = use_signal(|| true);
    let kicked_from_room = use_signal(|| None::<String>);
    let join_confirmation = use_signal(|| None::<super::state::VoiceJoinConfirmation>);
    let eviction_notice =
        use_signal(|| None::<cheenhub_contracts::realtime::VoicePresenceEvictionReason>);
    let pending_eviction = use_hook(|| Rc::new(RefCell::new(None)));
    let disconnected_presence = use_hook(|| Rc::new(RefCell::new(None)));
    let speaking_users = use_signal(Vec::new);
    let room_snapshots = use_signal(Vec::new);
    let remembered_room = use_signal(|| None::<VoiceRoomTarget>);
    let media_session = use_signal(|| None::<VoiceMediaSession>);
    let speaking_generations = use_hook(|| Rc::new(RefCell::new(HashMap::<String, u64>::new())));
    let join_generation = use_hook(|| Rc::new(Cell::<u64>::new(0)));
    let participant_video_streams = use_signal(Vec::new);
    let participant_video_subscribers = use_hook(|| Rc::new(RefCell::new(HashMap::new())));
    let participant_video_generations = use_hook(|| Rc::new(RefCell::new(HashMap::new())));
    let participant_video_blocked_streams = use_hook(|| Rc::new(RefCell::new(HashMap::new())));
    let participant_video = ParticipantVideoHandle::new(
        participant_video_streams,
        participant_video_subscribers,
        participant_video_generations,
        participant_video_blocked_streams,
        dioxus::core::current_scope_id(),
    );
    let participant_video_context = participant_video.clone();
    use_context_provider(move || participant_video_context.clone());
    let mut microphone_target_room = use_signal(|| None::<String>);
    let mut microphone_target_session_id = use_signal(|| None::<String>);
    let mut camera_target_room = use_signal(|| None::<LocalVideoTarget>);
    let mut screen_share_target_room = use_signal(|| None::<LocalVideoTarget>);
    let mut mic_paused_by_focus = use_signal(|| false);
    let mut mic_paused_by_deafen = use_signal(|| false);
    let voice_notification_sounds =
        use_hook(|| Rc::new(RefCell::new(VoiceNotificationSoundState::default())));
    let camera_notification_sounds =
        use_hook(|| Rc::new(RefCell::new(ToggleNotificationSoundState::default())));
    let screen_share_notification_sounds =
        use_hook(|| Rc::new(RefCell::new(ToggleNotificationSoundState::default())));
    let connection_notification_sounds =
        use_hook(|| Rc::new(RefCell::new(ConnectionNotificationSoundState::default())));
    let handle = VoiceConnectionHandle::new(VoiceConnectionParts {
        state,
        kicked_from_room,
        join_confirmation,
        eviction_notice,
        pending_eviction,
        disconnected_presence,
        speaking_users,
        room_snapshots,
        remembered_room,
        media_session,
        speaking_generations,
        join_generation: join_generation.clone(),
        realtime: realtime.clone(),
        microphone: microphone.clone(),
        playback: playback.clone(),
        current_user: current_user.clone(),
    });
    super::screen_audio::use_screen_audio(screen_share.clone(), realtime.clone(), handle.clone());
    let context_handle = handle.clone();
    use_context_provider(move || context_handle.clone());
    super::output_state::use_output_state_publication(
        state,
        join_generation.clone(),
        playback.clone(),
        microphone.clone(),
        realtime.clone(),
    );
    use_voice_network_quality_runtime(
        state,
        handle.clone(),
        current_user.id.clone(),
        realtime.clone(),
        network_quality,
    );

    voice_call_platform::use_background_realtime_activity(state, realtime.clone());
    super::voice_eviction_subscription::use_voice_eviction_subscription(
        realtime.clone(),
        handle.clone(),
    );

    super::voice_snapshot_subscription::use_voice_snapshot_subscription(
        realtime.clone(),
        handle.clone(),
        participant_video.clone(),
        playback.clone(),
    );
    let bitrate_realtime = realtime.clone();
    let bitrate_microphone = microphone.clone();
    let bitrate_state = state;
    use_hook(move || {
        spawn(async move {
            let mut events = realtime::subscribe_server_audio_bitrate(&bitrate_realtime);
            while let Some(event) = events.next().await {
                let Some(target) = bitrate_state().active_target() else {
                    continue;
                };
                if !matches!(target.kind, VoiceRoomTargetKind::Server)
                    || target.server_id != event.server_id
                {
                    continue;
                }
                debug!(
                    server_id = %event.server_id,
                    audio_bitrate_bps = event.audio_bitrate_bps,
                    "applying server audio bitrate to microphone"
                );
                bitrate_microphone.set_bitrate_bps(event.audio_bitrate_bps);
            }
        })
    });
    super::voice_audio_focus::use_voice_audio_focus(voice_audio_focused);
    super::inbound_audio::use_inbound_audio(
        realtime.clone(),
        playback.clone(),
        current_user.id.clone(),
        handle.clone(),
        voice_audio_focused,
    );
    super::video_runtime::use_video_datagram_runtime(
        state,
        realtime.clone(),
        participant_video.clone(),
        playback.clone(),
    );
    voice_recovery::use_voice_recovery(
        handle.clone(),
        realtime.clone(),
        state,
        playback.clone(),
        participant_video.clone(),
        connection_notification_sounds.clone(),
    );
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
    let indicator_system_tray = system_tray.clone();
    let indicator_microphone = microphone.clone();
    let indicator_playback = playback.clone();
    use_effect(move || {
        let connected = matches!(state(), VoiceConnectionState::Connected { .. });
        let microphone_muted = !indicator_microphone.voice_enabled();
        let output_muted = indicator_playback.is_muted();
        let speaking = connected
            && !microphone_muted
            && !output_muted
            && matches!(indicator_microphone.status(), MicrophoneStatus::Live)
            && indicator_microphone.level_active();
        indicator_system_tray.set_voice_indicator(
            connected,
            microphone_muted,
            output_muted,
            speaking,
        );
    });
    use_effect(move || {
        // Изменение deafen должно повторно пройти через lifecycle микрофона.
        let _ = playback.is_muted();
        match state() {
            VoiceConnectionState::Connected {
                target,
                participants,
            } => {
                let Some(session_id) =
                    effect_handle.outbound_session_id_for(&target.server_id, &target.room_id)
                else {
                    debug!(room_id = %target.room_id, "waiting for confirmed voice media session");
                    return;
                };
                let confirmed_session_id = session_id.to_string();
                if matches!(target.kind, VoiceRoomTargetKind::DirectMessage) {
                    microphone.set_bitrate_bps(cheenhub_contracts::media::VOICE_AUDIO_BITRATE_BPS);
                }
                if !platform_call_active() {
                    match voice_call_platform::set_voice_call_participating(true) {
                        Ok(()) => {
                            platform_call_active.set(true);
                            info!(
                                server_id = %target.server_id,
                                room_id = %target.room_id,
                                "acquired platform voice-call lifecycle"
                            );
                        }
                        Err(error) => {
                            warn!(
                                %error,
                                server_id = %target.server_id,
                                room_id = %target.room_id,
                                "failed to acquire platform voice-call lifecycle"
                            );
                        }
                    }
                }
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
                let output_muted = playback.is_muted();
                if output_muted {
                    if !mic_paused_by_deafen() {
                        mic_paused_by_deafen.set(true);
                    }
                    if matches!(
                        microphone.status_untracked(),
                        MicrophoneStatus::Live | MicrophoneStatus::Starting
                    ) {
                        microphone.stop();
                    }
                }
                if !voice_audio_focused() {
                    if !mic_paused_by_focus()
                        && matches!(
                            microphone.status_untracked(),
                            MicrophoneStatus::Live | MicrophoneStatus::Starting
                        )
                    {
                        mic_paused_by_focus.set(true);
                    }
                    microphone.stop();
                    playback.stop_voice_playback();
                    return;
                }

                playback.resume();
                let paused_by_focus = mic_paused_by_focus();
                if paused_by_focus {
                    mic_paused_by_focus.set(false);
                }
                let paused_by_deafen = mic_paused_by_deafen() && !output_muted;
                if paused_by_deafen {
                    mic_paused_by_deafen.set(false);
                }
                if microphone_target_room().as_deref() == Some(target.room_id.as_str()) {
                    let session_changed =
                        microphone_target_session_id() != Some(confirmed_session_id.clone());
                    if paused_by_focus || paused_by_deafen || session_changed {
                        if session_changed && !paused_by_focus {
                            microphone.stop();
                        }
                        microphone_target_session_id.set(Some(confirmed_session_id));
                        microphone_uplink::restart(
                            microphone.clone(),
                            realtime.clone(),
                            target.server_id.clone(),
                            target.room_id.clone(),
                            effect_handle.clone(),
                        );
                    }
                    info!(
                        target_kind = ?target.kind,
                        server_id = %target.server_id,
                        room_id = %target.room_id,
                        microphone_paused_by_focus = paused_by_focus,
                        microphone_paused_by_deafen = paused_by_deafen,
                        "voice media already targets active room"
                    );
                    return;
                }
                microphone_target_room.set(Some(target.room_id.clone()));
                microphone_target_session_id.set(Some(confirmed_session_id));
                info!(
                    target_kind = ?target.kind,
                    server_id = %target.server_id,
                    room_id = %target.room_id,
                    output_muted = playback.is_muted(),
                    "starting voice media for active room"
                );
                microphone_uplink::restart(
                    microphone.clone(),
                    realtime.clone(),
                    target.server_id,
                    target.room_id,
                    effect_handle.clone(),
                );
            }
            VoiceConnectionState::Disconnected
            | VoiceConnectionState::Connecting { .. }
            | VoiceConnectionState::Disconnecting { .. }
            | VoiceConnectionState::Error { .. } => {
                if mic_paused_by_deafen() {
                    mic_paused_by_deafen.set(false);
                }
                let previous_camera_target = camera_target_room();
                let previous_screen_share_target = screen_share_target_room();
                let had_camera_target = previous_camera_target.is_some();
                let had_screen_share_target = previous_screen_share_target.is_some();
                let had_microphone_target = microphone_target_room().is_some();
                if microphone_target_session_id().is_some() {
                    microphone_target_session_id.set(None);
                }
                let was_paused_by_focus = mic_paused_by_focus();

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
                if was_paused_by_focus {
                    mic_paused_by_focus.set(false);
                }
                if platform_call_active() {
                    match voice_call_platform::set_voice_call_participating(false) {
                        Ok(()) => {
                            platform_call_active.set(false);
                            info!("released platform voice-call lifecycle");
                        }
                        Err(error) => {
                            warn!(%error, "failed to release platform voice-call lifecycle");
                        }
                    }
                }
                effect_handle.clear_speaking_users();
                if effect_voice_sounds.borrow().is_current_user_connected() {
                    playback.stop_voice_playback();
                }
                effect_voice_sounds.borrow_mut().record_inactive(&playback);

                if had_camera_target
                    || had_screen_share_target
                    || had_microphone_target
                    || was_paused_by_focus
                {
                    info!(
                        microphone = had_microphone_target,
                        microphone_paused_by_focus = was_paused_by_focus,
                        camera = had_camera_target,
                        screen_share = had_screen_share_target,
                        "released local voice media resources"
                    );
                }
            }
        }
    });

    rsx! {
        DirectCallProvider {
            RoomInviteProvider {
                {children}
            }
        }
        VoiceSessionDialogs {}
        if let Some(room_name) = kicked_from_room() {
            KickedFromVoiceModal {
                room_name,
                on_close: move |_| handle.dismiss_kick_notification(),
            }
        }
    }
}

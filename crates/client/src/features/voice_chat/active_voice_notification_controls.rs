//! Владелец синхронизации активного звонка с системным Android-уведомлением.

use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::camera::CameraHandle;
use crate::features::microphone::{MicrophoneHandle, MicrophoneStatus};
use crate::features::realtime::RealtimeHandle;
use crate::features::screen_share::ScreenShareHandle;

use super::direct_call_state::DirectCallHandle;
use super::microphone_uplink;
use super::state::{VoiceConnectionHandle, VoiceConnectionState, VoiceRoomTargetKind};
use super::voice_call_platform::{
    self, ActiveVoiceNotification, VoiceNotificationAction, VoiceNotificationMicrophoneState,
    VoiceNotificationTargetKind,
};

/// Синхронизирует состояние звонка и исполняет команды системного уведомления.
#[component]
pub(super) fn ActiveVoiceNotificationControls() -> Element {
    let voice = use_context::<VoiceConnectionHandle>();
    let direct_call = use_context::<DirectCallHandle>();
    let microphone = use_context::<MicrophoneHandle>();
    let camera = use_context::<CameraHandle>();
    let screen_share = use_context::<ScreenShareHandle>();
    let realtime = use_context::<RealtimeHandle>();

    let notification_voice = voice.clone();
    let notification_microphone = microphone.clone();
    use_effect(move || {
        let notification = match notification_voice.state() {
            VoiceConnectionState::Connected { target, .. } => Some(ActiveVoiceNotification {
                target_kind: match target.kind {
                    VoiceRoomTargetKind::Server => VoiceNotificationTargetKind::ServerRoom,
                    VoiceRoomTargetKind::DirectMessage => VoiceNotificationTargetKind::DirectCall,
                },
                target_id: target.room_id,
                target_name: target.room_name,
                microphone: microphone_notification_state(notification_microphone.status()),
            }),
            VoiceConnectionState::Disconnected
            | VoiceConnectionState::Connecting { .. }
            | VoiceConnectionState::Disconnecting { .. }
            | VoiceConnectionState::Error { .. } => None,
        };
        voice_call_platform::update_active_voice_notification(notification);
    });

    use_hook(move || {
        spawn(async move {
            let mut actions = voice_call_platform::subscribe_voice_notification_actions();
            while let Some(action) = actions.next().await {
                let VoiceConnectionState::Connected { target, .. } = voice.state() else {
                    warn!(
                        ?action,
                        "ignored Android voice notification action without connected target"
                    );
                    continue;
                };
                match action {
                    VoiceNotificationAction::ToggleMicrophone => {
                        info!(
                            target_kind = ?target.kind,
                            server_id = %target.server_id,
                            room_id = %target.room_id,
                            "handling Android notification microphone toggle"
                        );
                        microphone_uplink::toggle(
                            microphone.clone(),
                            realtime.clone(),
                            target.server_id,
                            target.room_id,
                        );
                    }
                    VoiceNotificationAction::Leave => {
                        info!(
                            target_kind = ?target.kind,
                            server_id = %target.server_id,
                            room_id = %target.room_id,
                            "handling Android notification leave action"
                        );
                        microphone.stop();
                        camera.stop();
                        screen_share.stop();
                        if !direct_call.end_for_target(&target) {
                            voice.leave();
                        }
                    }
                }
            }
        })
    });

    rsx! {}
}

fn microphone_notification_state(status: MicrophoneStatus) -> VoiceNotificationMicrophoneState {
    match status {
        MicrophoneStatus::Idle => VoiceNotificationMicrophoneState::Off,
        MicrophoneStatus::Starting => VoiceNotificationMicrophoneState::Starting,
        MicrophoneStatus::Live => VoiceNotificationMicrophoneState::Live,
        MicrophoneStatus::PermissionDenied | MicrophoneStatus::Error(_) => {
            VoiceNotificationMicrophoneState::Unavailable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_microphone_status_for_notification() {
        assert_eq!(
            microphone_notification_state(MicrophoneStatus::Idle),
            VoiceNotificationMicrophoneState::Off
        );
        assert_eq!(
            microphone_notification_state(MicrophoneStatus::Starting),
            VoiceNotificationMicrophoneState::Starting
        );
        assert_eq!(
            microphone_notification_state(MicrophoneStatus::Live),
            VoiceNotificationMicrophoneState::Live
        );
        assert_eq!(
            microphone_notification_state(MicrophoneStatus::PermissionDenied),
            VoiceNotificationMicrophoneState::Unavailable
        );
    }
}

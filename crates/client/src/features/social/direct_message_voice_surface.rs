//! Голосовая область выбранного личного диалога.

use cheenhub_contracts::rest::DmConversationSummary;
use dioxus::prelude::*;

use crate::features::app::current_user::CurrentUserContext;
use crate::features::microphone::{MicrophoneHandle, MicrophoneStatus};
use crate::features::voice_chat::{
    VoiceConnectionHandle, VoiceConnectionState, VoiceControls, VoiceParticipantGrid,
    VoiceParticipantGridStatus,
};

use super::voice_target::direct_message_voice_target;

/// Рендерит активный голосовой звонок выбранного личного диалога.
#[component]
pub(crate) fn DirectMessageVoiceSurface(conversation: DmConversationSummary) -> Element {
    let current_user = use_context::<CurrentUserContext>().require_user();
    let voice = use_context::<VoiceConnectionHandle>();
    let microphone = use_context::<MicrophoneHandle>();
    let target = direct_message_voice_target(&conversation);
    let voice_state = voice.state();
    let selected_voice_active = voice_state
        .active_target()
        .is_some_and(|active| active.matches(&target));
    let selected_voice_participants = if selected_voice_active {
        voice_state.participants().to_vec()
    } else {
        Vec::new()
    };
    let selected_voice_status = match &voice_state {
        VoiceConnectionState::Connecting {
            target: connecting_target,
        } if connecting_target.matches(&target) => VoiceParticipantGridStatus::Connecting,
        VoiceConnectionState::Error {
            target: error_target,
            message,
        } if error_target
            .as_ref()
            .is_some_and(|error_target| error_target.matches(&target)) =>
        {
            VoiceParticipantGridStatus::Error {
                message: message.clone(),
            }
        }
        _ => VoiceParticipantGridStatus::Empty,
    };
    let mut selected_voice_speaking_user_ids = if selected_voice_active {
        voice.speaking_user_ids()
    } else {
        Vec::new()
    };
    if selected_voice_active
        && matches!(microphone.status(), MicrophoneStatus::Live)
        && microphone.level().active
        && !selected_voice_speaking_user_ids
            .iter()
            .any(|user_id| user_id == &current_user.id)
    {
        selected_voice_speaking_user_ids.push(current_user.id.clone());
    }
    let retry_voice = voice.clone();

    rsx! {
        div { class: "voice-room-surface relative flex min-h-0 flex-1 flex-col",
            VoiceParticipantGrid {
                server_id: target.server_id.clone(),
                room_id: target.room_id.clone(),
                participants: selected_voice_participants,
                speaking_user_ids: selected_voice_speaking_user_ids,
                status: selected_voice_status,
                can_kick_voice: false,
                on_retry: {
                    let retry_target = target.clone();
                    move |_| retry_voice.join(retry_target.clone())
                },
            }
            VoiceControls { target }
        }
    }
}

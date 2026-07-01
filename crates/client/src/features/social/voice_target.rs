//! Построение целей голосовых звонков личных диалогов.

use cheenhub_contracts::rest::DmConversationSummary;

use crate::features::voice_chat::VoiceRoomTarget;

pub(crate) fn direct_message_voice_target(conversation: &DmConversationSummary) -> VoiceRoomTarget {
    VoiceRoomTarget::direct_message(
        conversation.id.clone(),
        conversation.friend_nickname.clone(),
    )
}

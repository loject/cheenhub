//! Методы чтения состояния голосового подключения.

use cheenhub_contracts::realtime::VoiceRoomParticipant;

use super::{VoiceConnectionState, VoiceRoomTarget};

impl VoiceConnectionState {
    /// Возвращает, нужно ли показывать боковые элементы управления голосом.
    pub(crate) fn shows_sidebar_controls(&self) -> bool {
        !matches!(self, Self::Disconnected)
    }

    /// Возвращает, относится ли состояние к указанной комнате.
    pub(crate) fn is_active_room(&self, server_id: &str, room_id: &str) -> bool {
        self.active_target()
            .is_some_and(|target| target.server_id == server_id && target.room_id == room_id)
    }

    /// Возвращает, подключено ли состояние к указанной комнате.
    pub(crate) fn is_connected_room(&self, server_id: &str, room_id: &str) -> bool {
        matches!(
            self,
            Self::Connected { target, .. }
                if target.server_id == server_id && target.room_id == room_id
        )
    }

    /// Возвращает участников для отображения.
    pub(crate) fn participants(&self) -> &[VoiceRoomParticipant] {
        match self {
            Self::Connected { participants, .. } | Self::Disconnecting { participants, .. } => {
                participants
            }
            _ => &[],
        }
    }

    /// Возвращает активную цель, когда состояние привязано к комнате.
    pub(crate) fn active_target(&self) -> Option<VoiceRoomTarget> {
        match self {
            Self::Connecting { target }
            | Self::Connected { target, .. }
            | Self::Disconnecting { target, .. } => Some(target.clone()),
            Self::Error { target, .. } => target.clone(),
            Self::Disconnected => None,
        }
    }

    /// Возвращает, нужно ли временно блокировать вход в указанную голосовую цель.
    pub(crate) fn blocks_join_to(&self, target: &VoiceRoomTarget) -> bool {
        match self {
            Self::Connecting { .. } => true,
            Self::Disconnecting {
                target: leaving_target,
                ..
            } => leaving_target.matches(target),
            _ => false,
        }
    }

    pub(super) fn is_connected_to(&self, room: &VoiceRoomTarget) -> bool {
        matches!(
            self,
            Self::Connected { target, .. } if target.matches(room)
        )
    }

    pub(super) fn is_connecting_to(&self, room: &VoiceRoomTarget) -> bool {
        matches!(
            self,
            Self::Connecting { target } if target.matches(room)
        )
    }

    pub(super) fn is_disconnecting_from(&self, room: &VoiceRoomTarget) -> bool {
        matches!(
            self,
            Self::Disconnecting { target, .. } if target.matches(room)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{VoiceConnectionState, VoiceRoomTarget};

    #[test]
    fn leaving_server_does_not_block_joining_direct_message_voice() {
        let server = VoiceRoomTarget::server(
            "server".to_owned(),
            "server-room".to_owned(),
            "Голосовая".to_owned(),
        );
        let direct_message =
            VoiceRoomTarget::direct_message("conversation".to_owned(), "Друг".to_owned());
        let state = VoiceConnectionState::Disconnecting {
            target: server,
            participants: Vec::new(),
        };

        assert!(!state.blocks_join_to(&direct_message));
    }

    #[test]
    fn leaving_target_stays_blocked_until_its_leave_finishes() {
        let direct_message =
            VoiceRoomTarget::direct_message("conversation".to_owned(), "Друг".to_owned());
        let state = VoiceConnectionState::Disconnecting {
            target: direct_message.clone(),
            participants: Vec::new(),
        };

        assert!(state.blocks_join_to(&direct_message));
    }
}

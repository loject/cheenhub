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

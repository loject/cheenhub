//! Адресная обработка вытеснения голосовой сессии без выхода из аккаунта.

use cheenhub_contracts::realtime::{
    VoiceNetworkTargetKind, VoicePresenceEvicted, VoicePresenceEvictionReason,
};
use dioxus::prelude::*;

use super::{VoiceConnectionHandle, VoiceConnectionState, VoiceRoomTarget, VoiceRoomTargetKind};

/// Сессия, удаление которой уже показал снимок до адресного события сервера.
#[derive(Clone)]
pub(in crate::features::voice_chat) struct VoiceDisconnectionCandidate {
    pub(super) target: VoiceRoomTarget,
    pub(super) session_id: String,
    pub(super) joined_at: String,
}

impl VoiceConnectionHandle {
    /// Запоминает собственную сессию, когда снимок опередил точное вытеснение.
    pub(super) fn remember_disconnected_presence(
        &self,
        target: &VoiceRoomTarget,
        participants: &[cheenhub_contracts::realtime::VoiceRoomParticipant],
    ) {
        let Some(session_id) = self.media_session_id_for(target) else {
            return;
        };
        let Some(own) = participants.iter().find(|participant| {
            participant.user_id == self.current_user.id
                && participant.session_id == session_id.to_string()
        }) else {
            return;
        };
        *self.disconnected_presence.borrow_mut() = Some(VoiceDisconnectionCandidate {
            target: target.clone(),
            session_id: own.session_id.clone(),
            joined_at: own.joined_at.clone(),
        });
    }

    /// Исправляет предварительное уведомление, если адресное событие пришло позже.
    fn apply_late_eviction(&self, event: VoicePresenceEvicted) {
        let matches = self
            .disconnected_presence
            .borrow()
            .as_ref()
            .is_some_and(|candidate| {
                let kind = match candidate.target.kind {
                    VoiceRoomTargetKind::Server => VoiceNetworkTargetKind::Server,
                    VoiceRoomTargetKind::DirectMessage => VoiceNetworkTargetKind::DirectMessage,
                };
                event.target_kind == kind
                    && event.server_id == candidate.target.server_id
                    && event.room_id == candidate.target.room_id
                    && event.session_id == candidate.session_id
                    && !event.joined_at.is_empty()
                    && event.joined_at == candidate.joined_at
            });
        if !matches {
            return;
        }
        self.disconnected_presence.borrow_mut().take();
        self.take_remembered_room();
        warn!(
            session_id = %event.session_id,
            reason = ?event.reason,
            "correcting voice departure reason from late server eviction"
        );
        let mut kicked = self.kicked_from_room;
        kicked.set(None);
        let mut notice = self.eviction_notice;
        notice.set(Some(event.reason));
    }

    /// Завершает только своё голосовое присутствие по серверному уведомлению.
    pub(crate) fn apply_eviction(&self, event: VoicePresenceEvicted) {
        let current = self.state();
        let Some(target) = current.active_target() else {
            if matches!(current, VoiceConnectionState::Disconnected) {
                self.apply_late_eviction(event);
            }
            return;
        };
        let event_kind = match target.kind {
            VoiceRoomTargetKind::Server => VoiceNetworkTargetKind::Server,
            VoiceRoomTargetKind::DirectMessage => VoiceNetworkTargetKind::DirectMessage,
        };
        if event.target_kind != event_kind
            || target.server_id != event.server_id
            || target.room_id != event.room_id
        {
            return;
        }
        if matches!(current, VoiceConnectionState::Connecting { .. })
            || (matches!(current, VoiceConnectionState::Connected { .. })
                && self.media_session_id_for(&target).is_none())
        {
            // До join-ack известно лишь назначение комнаты, но не UUID своей сессии.
            *self.pending_eviction.borrow_mut() = Some(event);
            return;
        }
        let VoiceConnectionState::Connected { participants, .. } = current else {
            return;
        };
        if self.media_session_id_for(&target).map(|id| id.to_string())
            != Some(event.session_id.clone())
            || !eviction_matches_presence(&participants, &self.current_user.id, &event)
        {
            return;
        }
        self.finish_eviction(&target, event.reason);
    }

    /// Сверяет вытеснение, полученное до join-ack, с подтверждённым UUID.
    pub(super) fn apply_pending_eviction(
        &self,
        target: &VoiceRoomTarget,
        session_id: &str,
        participants: &[cheenhub_contracts::realtime::VoiceRoomParticipant],
    ) -> bool {
        let Some(event) = self.pending_eviction.borrow_mut().take() else {
            return false;
        };
        if event.session_id != session_id
            || event.server_id != target.server_id
            || event.room_id != target.room_id
            || !eviction_matches_presence(participants, &self.current_user.id, &event)
        {
            return false;
        }
        self.finish_eviction(target, event.reason);
        true
    }

    fn finish_eviction(&self, target: &VoiceRoomTarget, reason: VoicePresenceEvictionReason) {
        warn!(
            server_id = %target.server_id,
            room_id = %target.room_id,
            reason = ?reason,
            "server evicted this voice presence"
        );
        self.next_join_generation();
        self.take_remembered_room();
        self.disconnected_presence.borrow_mut().take();
        self.clear_media_session();
        let mut state = self.state;
        state.set(VoiceConnectionState::Disconnected);
        let mut notice = self.eviction_notice;
        notice.set(Some(reason));
    }

    /// Скрывает уведомление о смене голосового подключения.
    pub(crate) fn dismiss_eviction_notice(&self) {
        let mut notice = self.eviction_notice;
        notice.set(None);
    }
}

/// Не позволяет уведомлению о прежнем входе отключить новое присутствие того же транспорта.
fn eviction_matches_presence(
    participants: &[cheenhub_contracts::realtime::VoiceRoomParticipant],
    user_id: &str,
    event: &VoicePresenceEvicted,
) -> bool {
    !event.joined_at.is_empty()
        && participants.iter().any(|participant| {
            participant.user_id == user_id
                && participant.session_id == event.session_id
                && participant.joined_at == event.joined_at
        })
}

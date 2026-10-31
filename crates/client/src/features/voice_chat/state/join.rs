//! Жизненный цикл join-операций голосового подключения и обработка их результатов.

use cheenhub_contracts::realtime::RejectionCode;
use dioxus::prelude::*;

use crate::features::realtime::RealtimeError;
use uuid::Uuid;

use super::VoiceMediaSession;
use super::actions::{JoinedVoiceRoom, ensure_current_user_present};
use super::{
    JOIN_RESPONSE_TIMEOUT_MS, VoiceConnectionHandle, VoiceConnectionState, VoiceJoinConfirmation,
    VoiceRoomTarget,
};

impl VoiceConnectionHandle {
    /// Отменяет предложение сменить голосовое устройство.
    pub(crate) fn dismiss_join_confirmation(&self) {
        let mut confirmation = self.join_confirmation;
        confirmation.set(None);
    }

    /// Повторяет запрос с явным подтверждением отключения предыдущего устройства.
    pub(crate) fn confirm_join_conflict(&self) {
        let mut confirmation = self.join_confirmation;
        let Some(confirmation) = confirmation.write().take() else {
            return;
        };
        self.join_with_confirmation(confirmation.target, true);
    }

    /// Начинает новую join-операцию и возвращает её поколение.
    pub(super) fn next_join_generation(&self) -> u64 {
        let next = self.join_generation.get() + 1;
        self.join_generation.set(next);
        next
    }

    /// Возвращает, относится ли поколение к текущей join-операции.
    pub(super) fn is_current_join_generation(&self, generation: u64) -> bool {
        self.join_generation.get() == generation
    }

    /// Применяет ошибку join-запроса, если операция всё ещё актуальна.
    pub(super) fn apply_join_failure(
        &self,
        target: &VoiceRoomTarget,
        join_generation: u64,
        error: RealtimeError,
    ) {
        if !self.is_current_join_generation(join_generation)
            || !(self.state().is_connecting_to(target)
                || (self.state().is_connected_to(target)
                    && self.media_session_id_for(target).is_none()))
        {
            info!(
                target_kind = ?target.kind,
                server_id = %target.server_id,
                room_id = %target.room_id,
                join_generation,
                current_generation = self.join_generation.get(),
                "ignored stale voice room join failure"
            );
            return;
        }
        warn!(
            %error,
            target_kind = ?target.kind,
            server_id = %target.server_id,
            room_id = %target.room_id,
            "failed to join voice room"
        );
        let conflict = match error.rejection_code() {
            Some(RejectionCode::VoiceDeviceConfirmationRequired) => Some(false),
            Some(RejectionCode::VoiceSessionTakeoverConfirmationRequired) => Some(true),
            _ => None,
        };
        if let Some(same_auth_session) = conflict {
            let mut confirmation = self.join_confirmation;
            confirmation.set(Some(VoiceJoinConfirmation {
                target: target.clone(),
                same_auth_session,
            }));
        }
        let mut state = self.state;
        state.set(VoiceConnectionState::Error {
            target: Some(target.clone()),
            message: error.user_message().map(str::to_owned).unwrap_or_else(|| {
                "Не удалось подключиться к голосовой комнате. Проверь соединение и попробуй ещё раз.".to_owned()
            }),
        });
    }

    /// Применяет timeout join-запроса, если операция всё ещё актуальна.
    pub(super) fn apply_join_timeout(&self, target: &VoiceRoomTarget, join_generation: u64) {
        if !self.is_current_join_generation(join_generation)
            || !(self.state().is_connecting_to(target)
                || (self.state().is_connected_to(target)
                    && self.media_session_id_for(target).is_none()))
        {
            info!(
                target_kind = ?target.kind,
                server_id = %target.server_id,
                room_id = %target.room_id,
                join_generation,
                current_generation = self.join_generation.get(),
                "ignored stale voice room join timeout"
            );
            return;
        }
        warn!(
            timeout_ms = JOIN_RESPONSE_TIMEOUT_MS,
            target_kind = ?target.kind,
            server_id = %target.server_id,
            room_id = %target.room_id,
            "voice room join request timed out"
        );
        let mut state = self.state;
        state.set(VoiceConnectionState::Error {
            target: Some(target.clone()),
            message: "Сервер долго не отвечает. Проверь соединение и попробуй ещё раз.".to_owned(),
        });
    }

    /// Применяет ответ на запрос входа в зависимости от того, насколько он устарел.
    pub(super) fn complete_join(
        &self,
        target: &VoiceRoomTarget,
        join_generation: u64,
        joined: JoinedVoiceRoom,
    ) {
        let current_generation = self.join_generation.get();
        if join_generation != current_generation {
            info!(
                target_kind = ?target.kind,
                server_id = %target.server_id,
                room_id = %target.room_id,
                join_generation,
                current_generation,
                "ignored stale voice room join response"
            );
            return;
        }
        let current = self.state();
        let Some(session_id) = Uuid::parse_str(&joined.self_session_id)
            .ok()
            .filter(|session_id| !session_id.is_nil())
        else {
            warn!(
                target_kind = ?target.kind,
                server_id = %target.server_id,
                room_id = %target.room_id,
                "voice join returned invalid primary session id"
            );
            if current.is_connecting_to(target) || current.is_connected_to(target) {
                self.clear_media_session();
                let mut state = self.state;
                state.set(VoiceConnectionState::Error {
                    target: Some(target.clone()),
                    message: "Сервер не подтвердил голосовое подключение.".to_owned(),
                });
            }
            return;
        };
        if self.apply_pending_eviction(
            target,
            &joined.self_session_id,
            &joined.snapshot.participants,
        ) {
            return;
        }
        if current.is_connecting_to(target) {
            let mut media_session = self.media_session;
            media_session.set(Some(VoiceMediaSession {
                target: target.clone(),
                session_id,
            }));
            self.microphone.set_bitrate_bps(joined.audio_bitrate_bps);
            let mut snapshot = joined.snapshot;
            if let Some(cached) = self.room_participants(&target.server_id, &target.room_id)
                && cached.iter().any(|participant| {
                    participant.user_id == self.current_user.id
                        && participant.session_id == joined.self_session_id
                })
            {
                // Применяем более поздний ParticipantsChanged только для своей сессии.
                snapshot.participants = cached;
            }
            ensure_current_user_present(
                &mut snapshot.participants,
                &self.current_user,
                self.microphone.voice_enabled(),
                &joined.self_session_id,
            );
            self.apply_room_snapshot(snapshot.clone());
            info!(
                target_kind = ?target.kind,
                server_id = %target.server_id,
                room_id = %target.room_id,
                participants = snapshot.participants.len(),
                "joined voice room"
            );
            let mut state = self.state;
            state.set(VoiceConnectionState::Connected {
                target: target.clone(),
                participants: snapshot.participants,
            });
            return;
        }
        if current.is_connected_to(target) {
            // ParticipantsChanged мог подтвердить присутствие раньше ответа join.
            // Сохранение UUID дополнительно запускает ранее заблокированный медиатракт.
            let mut media_session = self.media_session;
            media_session.set(Some(VoiceMediaSession {
                target: target.clone(),
                session_id,
            }));
            info!(
                target_kind = ?target.kind,
                server_id = %target.server_id,
                room_id = %target.room_id,
                audio_bitrate_bps = joined.audio_bitrate_bps,
                "applying audio bitrate from late voice room join response"
            );
            self.microphone.set_bitrate_bps(joined.audio_bitrate_bps);
            return;
        }
        info!(
            target_kind = ?target.kind,
            server_id = %target.server_id,
            room_id = %target.room_id,
            "ignored stale voice room join response"
        );
    }
}

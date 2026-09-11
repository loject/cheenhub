//! Жизненный цикл join-операций голосового подключения и обработка их результатов.

use dioxus::prelude::*;

use crate::features::realtime::RealtimeError;

use super::actions::{JoinedVoiceRoom, ensure_current_user_present};
use super::{
    JOIN_RESPONSE_TIMEOUT_MS, VoiceConnectionHandle, VoiceConnectionState, VoiceRoomTarget,
};

impl VoiceConnectionHandle {
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
            || !self.state().is_connecting_to(target)
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
        let mut state = self.state;
        state.set(VoiceConnectionState::Error {
            target: Some(target.clone()),
            message: "Не удалось подключиться к голосовой комнате. Проверь соединение и попробуй ещё раз."
                .to_owned(),
        });
    }

    /// Применяет timeout join-запроса, если операция всё ещё актуальна.
    pub(super) fn apply_join_timeout(&self, target: &VoiceRoomTarget, join_generation: u64) {
        if !self.is_current_join_generation(join_generation)
            || !self.state().is_connecting_to(target)
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
        if current.is_connecting_to(target) {
            self.microphone.set_bitrate_bps(joined.audio_bitrate_bps);
            let mut snapshot = joined.snapshot;
            ensure_current_user_present(&mut snapshot.participants, &self.current_user);
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
            // Join уже подтверждён более свежим ParticipantsChanged: участники и кэш
            // комнат не трогаем, но целевой bitrate сервера из join-response применяем.
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

//! Shared voice connection state.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use cheenhub_contracts::realtime::{
    VoicePresenceEvicted, VoicePresenceEvictionReason, VoiceRoomParticipant,
};
use cheenhub_contracts::rest::AuthUser;
use dioxus::prelude::*;
use futures_util::future::{Either, FutureExt, select};
use uuid::Uuid;

use crate::features::audio_playback::AudioPlaybackHandle;
use crate::features::microphone::MicrophoneHandle;
use crate::features::realtime::RealtimeHandle;
use crate::features::runtime::sleep_ms;

use super::realtime;
use super::room_presence::VoiceRoomParticipants;
use super::speaking::{self, SpeakingUserActivity};

mod actions;
mod eviction;
mod join;
mod media;
mod rooms;
mod status;
mod target;

use actions::{join_target, leave_target};
use eviction::VoiceDisconnectionCandidate;
pub(crate) use target::{VoiceRoomTarget, VoiceRoomTargetKind};

const JOIN_RESPONSE_TIMEOUT_MS: u32 = 12_000;

/// Current voice connection state for this client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VoiceConnectionState {
    /// No active voice presence.
    Disconnected,
    /// Join request is in flight.
    Connecting {
        /// Target room.
        target: VoiceRoomTarget,
    },
    /// Joined one voice room.
    Connected {
        /// Active room.
        target: VoiceRoomTarget,
        /// Current participants.
        participants: Vec<VoiceRoomParticipant>,
    },
    /// Leave request is in flight.
    Disconnecting {
        /// Room being left.
        target: VoiceRoomTarget,
        /// Last known participants.
        participants: Vec<VoiceRoomParticipant>,
    },
    /// Last voice action failed.
    Error {
        /// Target room when the error is room-scoped.
        target: Option<VoiceRoomTarget>,
        /// User-facing error message.
        message: String,
    },
}

/// Конфликт устройств, для которого пользователь может подтвердить повторный вход.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VoiceJoinConfirmation {
    pub(crate) target: VoiceRoomTarget,
    pub(crate) same_auth_session: bool,
}

/// Подтверждённый сервером источник исходящих медиадатаграмм.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VoiceMediaSession {
    /// Цель входа, для которой действителен UUID подключения.
    pub(crate) target: VoiceRoomTarget,
    /// UUID основного голосового подключения, а не отдельного microphone worker.
    pub(crate) session_id: Uuid,
}

/// Context handle used by voice chat UI surfaces.
#[derive(Clone)]
pub(crate) struct VoiceConnectionHandle {
    /// Shared voice state signal.
    pub(crate) state: Signal<VoiceConnectionState>,
    /// Room name the user was kicked from, set when a kick is detected.
    pub(crate) kicked_from_room: Signal<Option<String>>,
    pub(crate) join_confirmation: Signal<Option<VoiceJoinConfirmation>>,
    pub(crate) eviction_notice: Signal<Option<VoicePresenceEvictionReason>>,
    pending_eviction: Rc<RefCell<Option<VoicePresenceEvicted>>>,
    disconnected_presence: Rc<RefCell<Option<VoiceDisconnectionCandidate>>>,
    speaking_users: Signal<Vec<SpeakingUserActivity>>,
    room_snapshots: Signal<Vec<VoiceRoomParticipants>>,
    /// Комната, в которой пользователь был до последнего обрыва realtime.
    remembered_room: Signal<Option<VoiceRoomTarget>>,
    /// UUID активной медиасессии доступен только после ответа join.
    media_session: Signal<Option<VoiceMediaSession>>,
    speaking_generations: Rc<RefCell<HashMap<String, u64>>>,
    /// Поколение текущей операции join; ответы старых операций игнорируются.
    join_generation: Rc<Cell<u64>>,
    realtime: RealtimeHandle,
    microphone: MicrophoneHandle,
    playback: AudioPlaybackHandle,
    current_user: AuthUser,
}

/// Состояние и зависимости для создания контекста голосового подключения.
pub(super) struct VoiceConnectionParts {
    /// Основное состояние голосового подключения.
    pub(super) state: Signal<VoiceConnectionState>,
    /// Название комнаты, из которой исключили пользователя.
    pub(super) kicked_from_room: Signal<Option<String>>,
    /// Конфликт входа, требующий подтверждения отключения устройства.
    pub(super) join_confirmation: Signal<Option<VoiceJoinConfirmation>>,
    /// Уведомление о вытеснении именно этого голосового подключения.
    pub(super) eviction_notice: Signal<Option<VoicePresenceEvictionReason>>,
    /// Вытеснение, пришедшее до ответа сервера с UUID голосовой сессии.
    pub(super) pending_eviction: Rc<RefCell<Option<VoicePresenceEvicted>>>,
    /// Завершённая сессия, для которой ещё может прийти точная причина вытеснения.
    pub(super) disconnected_presence: Rc<RefCell<Option<VoiceDisconnectionCandidate>>>,
    /// Сигналы активности речи участников.
    pub(super) speaking_users: Signal<Vec<SpeakingUserActivity>>,
    /// Кэш снимков участников комнат.
    pub(super) room_snapshots: Signal<Vec<VoiceRoomParticipants>>,
    /// Комната, в которой пользователь был до последнего обрыва realtime.
    pub(super) remembered_room: Signal<Option<VoiceRoomTarget>>,
    /// Подтверждённое основное голосовое подключение.
    pub(super) media_session: Signal<Option<VoiceMediaSession>>,
    /// Поколения таймеров активности речи.
    pub(super) speaking_generations: Rc<RefCell<HashMap<String, u64>>>,
    /// Счётчик поколений join-операций для отброса устаревших ответов.
    pub(super) join_generation: Rc<Cell<u64>>,
    /// Realtime-контекст клиента.
    pub(super) realtime: RealtimeHandle,
    /// Контекст захвата микрофона.
    pub(super) microphone: MicrophoneHandle,
    /// Контекст входящего воспроизведения голосовой сессии.
    pub(super) playback: AudioPlaybackHandle,
    /// Текущий пользователь.
    pub(super) current_user: AuthUser,
}

impl VoiceConnectionHandle {
    /// Builds a voice connection handle.
    pub(super) fn new(parts: VoiceConnectionParts) -> Self {
        Self {
            state: parts.state,
            kicked_from_room: parts.kicked_from_room,
            join_confirmation: parts.join_confirmation,
            eviction_notice: parts.eviction_notice,
            pending_eviction: parts.pending_eviction,
            disconnected_presence: parts.disconnected_presence,
            speaking_users: parts.speaking_users,
            room_snapshots: parts.room_snapshots,
            remembered_room: parts.remembered_room,
            media_session: parts.media_session,
            speaking_generations: parts.speaking_generations,
            join_generation: parts.join_generation,
            realtime: parts.realtime,
            microphone: parts.microphone,
            playback: parts.playback,
            current_user: parts.current_user,
        }
    }

    /// Clears the kick notification after the user acknowledges it.
    pub(crate) fn dismiss_kick_notification(&self) {
        let mut kicked = self.kicked_from_room;
        kicked.set(None);
    }

    /// Reads the current voice connection state.
    pub(crate) fn state(&self) -> VoiceConnectionState {
        (self.state)()
    }

    /// Возвращает, выключено ли входящее голосовое воспроизведение.
    pub(crate) fn output_muted(&self) -> bool {
        self.playback.is_muted()
    }

    /// Меняет состояние входящего звука через принадлежащий голосовому контексту playback.
    pub(crate) fn set_output_muted(&self, muted: bool) {
        if muted {
            self.microphone.stop();
        }
        self.playback.set_muted(muted);
    }

    /// Returns the authenticated current user identifier.
    pub(crate) fn current_user_id(&self) -> &str {
        &self.current_user.id
    }

    /// Returns user identifiers currently marked as speaking.
    pub(crate) fn speaking_user_ids(&self) -> Vec<String> {
        speaking::user_ids(self.speaking_users)
    }

    /// Возвращает независимые метки речи голосовых сессий.
    pub(crate) fn speaking_sessions(&self) -> Vec<(String, Option<Uuid>)> {
        speaking::sessions(self.speaking_users)
    }

    /// Marks one user as speaking until no new voice frame refreshes the marker.
    pub(crate) fn mark_user_speaking(&self, user_id: String, session_id: Option<Uuid>) {
        speaking::mark_user_speaking(
            self.speaking_users,
            self.speaking_generations.clone(),
            user_id,
            session_id,
        );
    }

    /// Запоминает комнату, в которой пользователь был до обрыва realtime.
    ///
    /// Значение используется восстановлением после переподключения, поэтому
    /// оно не должно зависеть от текущего состояния соединения.
    pub(crate) fn remember_room(&mut self, target: VoiceRoomTarget) {
        let mut remembered_room = self.remembered_room;
        remembered_room.set(Some(target));
    }

    /// Забирает цель восстановления, очищая её перед повторным входом.
    ///
    /// Каждая цель используется один раз; ручной выход также забирает её,
    /// чтобы последующее переподключение не вернуло пользователя в старую комнату.
    pub(super) fn take_remembered_room(&self) -> Option<VoiceRoomTarget> {
        let mut remembered_room = self.remembered_room;
        remembered_room.write().take()
    }

    /// Очищает все удалённые speaking-индикаторы.
    pub(crate) fn clear_speaking_users(&self) {
        speaking::clear_speaking_users(self.speaking_users, self.speaking_generations.clone());
    }

    /// Joins one room, leaving the previous room first when needed.
    pub(crate) fn join(&self, target: VoiceRoomTarget) {
        self.join_with_confirmation(target, false);
    }

    fn join_with_confirmation(&self, target: VoiceRoomTarget, confirmed: bool) {
        let current = self.state();
        if current.is_connected_to(&target) || current.is_connecting_to(&target) {
            return;
        }
        self.disconnected_presence.borrow_mut().take();
        self.clear_media_session();
        let mut join_confirmation = self.join_confirmation;
        join_confirmation.set(None);

        let previous = if matches!(&current, VoiceConnectionState::Disconnecting { .. }) {
            info!(
                target_kind = ?target.kind,
                server_id = %target.server_id,
                room_id = %target.room_id,
                "joining new voice target while previous leave is still in flight"
            );
            None
        } else {
            current.active_target()
        };
        let realtime = self.realtime.clone();
        let handle = self.clone();
        let join_generation = self.next_join_generation();
        let mut state = self.state;
        state.set(VoiceConnectionState::Connecting {
            target: target.clone(),
        });
        info!(
            target_kind = ?target.kind,
            server_id = %target.server_id,
            room_id = %target.room_id,
            join_generation,
            "joining voice room"
        );

        if let Some(previous) = previous.filter(|previous| !previous.matches(&target)) {
            let leave_realtime = realtime.clone();
            spawn(async move {
                info!(
                    target_kind = ?previous.kind,
                    server_id = %previous.server_id,
                    room_id = %previous.room_id,
                    "leaving previous voice room while switching"
                );
                if let Err(error) = leave_target(&leave_realtime, &previous).await {
                    warn!(
                        %error,
                        target_kind = ?previous.kind,
                        server_id = %previous.server_id,
                        room_id = %previous.room_id,
                        "failed to leave previous voice room while switching"
                    );
                }
            });
        }

        spawn(async move {
            info!(
                target_kind = ?target.kind,
                server_id = %target.server_id,
                room_id = %target.room_id,
                "voice room join task started"
            );
            info!(
                target_kind = ?target.kind,
                server_id = %target.server_id,
                room_id = %target.room_id,
                "requesting voice room join"
            );
            let join = join_target(
                &realtime,
                &target,
                handle.microphone.voice_enabled_untracked() && !handle.playback.is_muted(),
                confirmed,
            );
            match select(
                join.boxed_local(),
                sleep_ms(JOIN_RESPONSE_TIMEOUT_MS).boxed_local(),
            )
            .await
            {
                Either::Left((Ok(joined), _)) => {
                    handle.complete_join(&target, join_generation, joined);
                }
                Either::Left((Err(error), _)) => {
                    handle.apply_join_failure(&target, join_generation, error);
                }
                Either::Right((_, _)) => {
                    handle.apply_join_timeout(&target, join_generation);
                }
            }
        });
    }

    /// Kicks one participant from the active voice room.
    pub(crate) fn kick_member(&self, server_id: String, room_id: String, user_id: String) {
        let realtime = self.realtime.clone();
        spawn(async move {
            if let Err(error) =
                realtime::kick_voice_member(&realtime, server_id, room_id, user_id).await
            {
                warn!(%error, "failed to kick voice member");
            }
        });
    }

    /// Leaves the active voice room.
    pub(crate) fn leave(&self) {
        if let Some(target) = self.take_remembered_room() {
            info!(server_id = %target.server_id, room_id = %target.room_id,
                "cancelled voice room recovery on manual leave");
        }
        // Инвалидируем текущую join-операцию, чтобы ответ отменённого входа
        // не мог примениться после leave.
        self.next_join_generation();
        self.disconnected_presence.borrow_mut().take();
        self.clear_media_session();
        let current = self.state();
        let Some(target) = current.active_target() else {
            let mut state = self.state;
            state.set(VoiceConnectionState::Disconnected);
            return;
        };
        let participants = current.participants().to_vec();
        let realtime = self.realtime.clone();
        let mut state = self.state;
        state.set(VoiceConnectionState::Disconnecting {
            target: target.clone(),
            participants,
        });

        spawn(async move {
            match leave_target(&realtime, &target).await {
                Ok(_) => {
                    if state().is_disconnecting_from(&target) {
                        state.set(VoiceConnectionState::Disconnected);
                    } else {
                        info!(
                            server_id = %target.server_id,
                            room_id = %target.room_id,
                            "ignored stale voice room leave response"
                        );
                    }
                }
                Err(error) => {
                    if !state().is_disconnecting_from(&target) {
                        info!(
                            server_id = %target.server_id,
                            room_id = %target.room_id,
                            "ignored stale voice room leave failure"
                        );
                        return;
                    }
                    warn!(
                        %error,
                        server_id = %target.server_id,
                        room_id = %target.room_id,
                        "failed to leave voice room"
                    );
                    state.set(VoiceConnectionState::Error {
                        target: Some(target),
                        message: error.to_string(),
                    });
                }
            }
        });
    }

    /// Applies a participant snapshot event.
    pub(crate) fn apply_snapshot(&self, snapshot: cheenhub_contracts::realtime::VoiceRoomSnapshot) {
        self.apply_room_snapshot(snapshot.clone());
        let current = self.state();
        let Some(target) = current.active_target() else {
            return;
        };
        if target.server_id != snapshot.server_id || target.room_id != snapshot.room_id {
            return;
        }

        let current_user_id = &self.current_user.id;
        let mut state = self.state;
        state.set(match current {
            VoiceConnectionState::Connecting { target } => {
                // Presence другого устройства того же аккаунта не подтверждает наш join.
                // Сохраняем список в кэше; вход завершает только ответ с UUID сессии.
                VoiceConnectionState::Connecting { target }
            }
            VoiceConnectionState::Connected {
                target,
                participants,
            } => {
                let session_id = self.media_session_id_for(&target).map(|id| id.to_string());
                let has_session_ids = snapshot
                    .participants
                    .iter()
                    .any(|participant| !participant.session_id.is_empty());
                let own_present = if has_session_ids {
                    session_id.as_ref().map_or_else(
                        || {
                            snapshot
                                .participants
                                .iter()
                                .any(|p| &p.user_id == current_user_id)
                        },
                        |id| snapshot.participants.iter().any(|p| &p.session_id == id),
                    )
                } else {
                    snapshot
                        .participants
                        .iter()
                        .any(|p| &p.user_id == current_user_id)
                };
                if own_present {
                    VoiceConnectionState::Connected {
                        target,
                        participants: snapshot.participants,
                    }
                } else {
                    self.remember_disconnected_presence(&target, &participants);
                    self.clear_media_session();
                    if snapshot
                        .participants
                        .iter()
                        .any(|p| &p.user_id == current_user_id)
                    {
                        let mut notice = self.eviction_notice;
                        notice.set(Some(VoicePresenceEvictionReason::AnotherDeviceTransition));
                    } else {
                        let mut kicked = self.kicked_from_room;
                        kicked.set(Some(target.room_name.clone()));
                    }
                    VoiceConnectionState::Disconnected
                }
            }
            VoiceConnectionState::Disconnecting { target, .. } => {
                VoiceConnectionState::Disconnecting {
                    target,
                    participants: snapshot.participants,
                }
            }
            VoiceConnectionState::Error { target, message } => {
                VoiceConnectionState::Error { target, message }
            }
            VoiceConnectionState::Disconnected => VoiceConnectionState::Disconnected,
        });
    }
}

#[cfg(test)]
mod tests;

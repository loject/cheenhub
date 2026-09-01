//! Shared voice connection state.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use cheenhub_contracts::realtime::VoiceRoomParticipant;
use cheenhub_contracts::rest::AuthUser;
use dioxus::prelude::*;
use futures_util::future::{Either, FutureExt, select};

use crate::features::microphone::MicrophoneHandle;
use crate::features::realtime::RealtimeHandle;
use crate::features::runtime::sleep_ms;

use super::realtime;
use super::room_presence::VoiceRoomParticipants;
use super::speaking::{self, SpeakingUserActivity};

mod actions;
mod rooms;
mod status;
mod target;

use actions::{ensure_current_user_present, join_target, leave_target};
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

/// Context handle used by voice chat UI surfaces.
#[derive(Clone)]
pub(crate) struct VoiceConnectionHandle {
    /// Shared voice state signal.
    pub(crate) state: Signal<VoiceConnectionState>,
    /// Room name the user was kicked from, set when a kick is detected.
    pub(crate) kicked_from_room: Signal<Option<String>>,
    speaking_users: Signal<Vec<SpeakingUserActivity>>,
    room_snapshots: Signal<Vec<VoiceRoomParticipants>>,
    speaking_generations: Rc<RefCell<HashMap<String, u64>>>,
    realtime: RealtimeHandle,
    microphone: MicrophoneHandle,
    current_user: AuthUser,
}

/// Состояние и зависимости для создания контекста голосового подключения.
pub(super) struct VoiceConnectionParts {
    /// Основное состояние голосового подключения.
    pub(super) state: Signal<VoiceConnectionState>,
    /// Название комнаты, из которой исключили пользователя.
    pub(super) kicked_from_room: Signal<Option<String>>,
    /// Сигналы активности речи участников.
    pub(super) speaking_users: Signal<Vec<SpeakingUserActivity>>,
    /// Кэш снимков участников комнат.
    pub(super) room_snapshots: Signal<Vec<VoiceRoomParticipants>>,
    /// Поколения таймеров активности речи.
    pub(super) speaking_generations: Rc<RefCell<HashMap<String, u64>>>,
    /// Realtime-контекст клиента.
    pub(super) realtime: RealtimeHandle,
    /// Контекст захвата микрофона.
    pub(super) microphone: MicrophoneHandle,
    /// Текущий пользователь.
    pub(super) current_user: AuthUser,
}

impl VoiceConnectionHandle {
    /// Builds a voice connection handle.
    pub(super) fn new(parts: VoiceConnectionParts) -> Self {
        Self {
            state: parts.state,
            kicked_from_room: parts.kicked_from_room,
            speaking_users: parts.speaking_users,
            room_snapshots: parts.room_snapshots,
            speaking_generations: parts.speaking_generations,
            realtime: parts.realtime,
            microphone: parts.microphone,
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

    /// Returns the authenticated current user identifier.
    pub(crate) fn current_user_id(&self) -> &str {
        &self.current_user.id
    }

    /// Returns user identifiers currently marked as speaking.
    pub(crate) fn speaking_user_ids(&self) -> Vec<String> {
        speaking::user_ids(self.speaking_users)
    }

    /// Marks one user as speaking until no new voice frame refreshes the marker.
    pub(crate) fn mark_user_speaking(&self, user_id: String) {
        speaking::mark_user_speaking(
            self.speaking_users,
            self.speaking_generations.clone(),
            user_id,
        );
    }

    /// Clears all remote speaking indicators.
    pub(crate) fn clear_speaking_users(&self) {
        speaking::clear_speaking_users(self.speaking_users, self.speaking_generations.clone());
    }

    /// Joins one room, leaving the previous room first when needed.
    pub(crate) fn join(&self, target: VoiceRoomTarget) {
        let current = self.state();
        if current.is_connected_to(&target) || current.is_connecting_to(&target) {
            return;
        }

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
        let microphone = self.microphone.clone();
        let handle = self.clone();
        let mut state = self.state;
        let user = self.current_user.clone();
        state.set(VoiceConnectionState::Connecting {
            target: target.clone(),
        });
        info!(
            target_kind = ?target.kind,
            server_id = %target.server_id,
            room_id = %target.room_id,
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
            let join = join_target(&realtime, &target);
            match select(
                join.boxed_local(),
                sleep_ms(JOIN_RESPONSE_TIMEOUT_MS).boxed_local(),
            )
            .await
            {
                Either::Left((Ok(joined), _)) => {
                    if !state().is_connecting_to(&target) {
                        info!(
                            target_kind = ?target.kind,
                            server_id = %target.server_id,
                            room_id = %target.room_id,
                            "ignored stale voice room join response"
                        );
                        return;
                    }
                    microphone.set_bitrate_bps(joined.audio_bitrate_bps);
                    let mut snapshot = joined.snapshot;
                    ensure_current_user_present(&mut snapshot.participants, &user);
                    handle.apply_room_snapshot(snapshot.clone());
                    info!(
                        target_kind = ?target.kind,
                        server_id = %target.server_id,
                        room_id = %target.room_id,
                        participants = snapshot.participants.len(),
                        "joined voice room"
                    );
                    state.set(VoiceConnectionState::Connected {
                        target: target.clone(),
                        participants: snapshot.participants,
                    });
                }
                Either::Left((Err(error), _)) => {
                    if !state().is_connecting_to(&target) {
                        info!(
                            target_kind = ?target.kind,
                            server_id = %target.server_id,
                            room_id = %target.room_id,
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
                    state.set(VoiceConnectionState::Error {
                        target: Some(target.clone()),
                        message: "Не удалось подключиться к голосовой комнате. Проверь соединение и попробуй ещё раз."
                            .to_owned(),
                    });
                }
                Either::Right((_, _)) => {
                    if !state().is_connecting_to(&target) {
                        info!(
                            target_kind = ?target.kind,
                            server_id = %target.server_id,
                            room_id = %target.room_id,
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
                    state.set(VoiceConnectionState::Error {
                        target: Some(target.clone()),
                        message: "Сервер долго не отвечает. Проверь соединение и попробуй ещё раз."
                            .to_owned(),
                    });
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
                VoiceConnectionState::Connecting { target }
            }
            VoiceConnectionState::Connected { target, .. } => {
                if snapshot
                    .participants
                    .iter()
                    .any(|p| &p.user_id == current_user_id)
                {
                    VoiceConnectionState::Connected {
                        target,
                        participants: snapshot.participants,
                    }
                } else {
                    let mut kicked = self.kicked_from_room;
                    kicked.set(Some(target.room_name.clone()));
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

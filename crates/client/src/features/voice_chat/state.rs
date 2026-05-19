//! Shared voice connection state.

use cheenhub_contracts::realtime::VoiceRoomParticipant;
use cheenhub_contracts::rest::AuthUser;
use dioxus::prelude::*;
use futures_util::future::{Either, FutureExt, select};
use gloo_timers::future::TimeoutFuture;

use crate::features::realtime::RealtimeHandle;

use super::realtime;

const SPEAKING_RELEASE_TIMEOUT_MS: u32 = 450;
const JOIN_RESPONSE_TIMEOUT_MS: u32 = 12_000;

/// Voice-capable room target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VoiceRoomTarget {
    /// Server identifier.
    pub(crate) server_id: String,
    /// Room identifier.
    pub(crate) room_id: String,
    /// Human-readable room name.
    pub(crate) room_name: String,
}

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
    speaking_users: Signal<Vec<SpeakingUserActivity>>,
    realtime: RealtimeHandle,
    current_user: AuthUser,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpeakingUserActivity {
    /// Participant identifier.
    user_id: String,
    /// Monotonic marker generation used to ignore stale release timers.
    generation: u64,
}

impl VoiceConnectionHandle {
    /// Builds a voice connection handle.
    pub(crate) fn new(
        state: Signal<VoiceConnectionState>,
        speaking_users: Signal<Vec<SpeakingUserActivity>>,
        realtime: RealtimeHandle,
        current_user: AuthUser,
    ) -> Self {
        Self {
            state,
            speaking_users,
            realtime,
            current_user,
        }
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
        (self.speaking_users)()
            .into_iter()
            .map(|activity| activity.user_id)
            .collect()
    }

    /// Marks one user as speaking until no new voice frame refreshes the marker.
    pub(crate) fn mark_user_speaking(&self, user_id: String) {
        let mut next_users = (self.speaking_users)();
        let generation = match next_users
            .iter_mut()
            .find(|activity| activity.user_id == user_id)
        {
            Some(activity) => {
                activity.generation = activity.generation.saturating_add(1);
                activity.generation
            }
            None => {
                next_users.push(SpeakingUserActivity {
                    user_id: user_id.clone(),
                    generation: 0,
                });
                0
            }
        };
        let mut speaking_users = self.speaking_users;
        speaking_users.set(next_users);

        spawn(async move {
            TimeoutFuture::new(SPEAKING_RELEASE_TIMEOUT_MS).await;
            let mut next_users = speaking_users();
            let previous_len = next_users.len();
            next_users.retain(|activity| {
                activity.user_id != user_id || activity.generation != generation
            });
            if next_users.len() != previous_len {
                speaking_users.set(next_users);
            }
        });
    }

    /// Clears all remote speaking indicators.
    pub(crate) fn clear_speaking_users(&self) {
        let mut speaking_users = self.speaking_users;
        speaking_users.set(Vec::new());
    }

    /// Joins one room, leaving the previous room first when needed.
    pub(crate) fn join(&self, target: VoiceRoomTarget) {
        let current = self.state();
        if current.is_connected_to(&target) || current.is_connecting_to(&target) {
            return;
        }

        let previous = current.active_target();
        let realtime = self.realtime.clone();
        let mut state = self.state;
        let user = self.current_user.clone();
        state.set(VoiceConnectionState::Connecting {
            target: target.clone(),
        });
        info!(
            server_id = %target.server_id,
            room_id = %target.room_id,
            "joining voice room"
        );

        spawn(async move {
            if let Some(previous) = previous
                && previous.room_id != target.room_id
                && let Err(error) = realtime::leave_room(
                    &realtime,
                    previous.server_id.clone(),
                    previous.room_id.clone(),
                )
                .await
            {
                warn!(
                    %error,
                    server_id = %previous.server_id,
                    room_id = %previous.room_id,
                    "failed to leave previous voice room before switching"
                );
            }

            let join =
                realtime::join_room(&realtime, target.server_id.clone(), target.room_id.clone());
            match select(
                join.boxed_local(),
                TimeoutFuture::new(JOIN_RESPONSE_TIMEOUT_MS).boxed_local(),
            )
            .await
            {
                Either::Left((Ok(mut snapshot), _)) => {
                    ensure_current_user_present(&mut snapshot.participants, &user);
                    info!(
                        server_id = %target.server_id,
                        room_id = %target.room_id,
                        participants = snapshot.participants.len(),
                        "joined voice room"
                    );
                    state.set(VoiceConnectionState::Connected {
                        target,
                        participants: snapshot.participants,
                    });
                }
                Either::Left((Err(error), _)) => {
                    warn!(
                        %error,
                        server_id = %target.server_id,
                        room_id = %target.room_id,
                        "failed to join voice room"
                    );
                    state.set(VoiceConnectionState::Error {
                        target: Some(target),
                        message: "Не удалось подключиться к голосовой комнате. Проверь соединение и попробуй ещё раз."
                            .to_owned(),
                    });
                }
                Either::Right((_, _)) => {
                    warn!(
                        timeout_ms = JOIN_RESPONSE_TIMEOUT_MS,
                        server_id = %target.server_id,
                        room_id = %target.room_id,
                        "voice room join request timed out"
                    );
                    state.set(VoiceConnectionState::Error {
                        target: Some(target),
                        message: "Сервер долго не отвечает. Проверь соединение и попробуй ещё раз."
                            .to_owned(),
                    });
                }
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
            match realtime::leave_room(&realtime, target.server_id.clone(), target.room_id.clone())
                .await
            {
                Ok(_) => state.set(VoiceConnectionState::Disconnected),
                Err(error) => {
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
        let current = self.state();
        let Some(target) = current.active_target() else {
            return;
        };
        if target.server_id != snapshot.server_id || target.room_id != snapshot.room_id {
            return;
        }

        let mut state = self.state;
        state.set(match current {
            VoiceConnectionState::Connecting { target } => {
                VoiceConnectionState::Connecting { target }
            }
            VoiceConnectionState::Connected { target, .. } => VoiceConnectionState::Connected {
                target,
                participants: snapshot.participants,
            },
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

impl VoiceConnectionState {
    /// Returns whether the state should show sidebar controls.
    pub(crate) fn shows_sidebar_controls(&self) -> bool {
        !matches!(self, Self::Disconnected)
    }

    /// Returns whether this state belongs to one room.
    pub(crate) fn is_active_room(&self, server_id: &str, room_id: &str) -> bool {
        self.active_target()
            .is_some_and(|target| target.server_id == server_id && target.room_id == room_id)
    }

    /// Returns whether this state is connected to one room.
    pub(crate) fn is_connected_room(&self, server_id: &str, room_id: &str) -> bool {
        matches!(
            self,
            Self::Connected { target, .. }
                if target.server_id == server_id && target.room_id == room_id
        )
    }

    /// Returns participants for display.
    pub(crate) fn participants(&self) -> &[VoiceRoomParticipant] {
        match self {
            Self::Connected { participants, .. } | Self::Disconnecting { participants, .. } => {
                participants
            }
            _ => &[],
        }
    }

    /// Returns active target when the state is room-scoped.
    pub(crate) fn active_target(&self) -> Option<VoiceRoomTarget> {
        match self {
            Self::Connecting { target }
            | Self::Connected { target, .. }
            | Self::Disconnecting { target, .. } => Some(target.clone()),
            Self::Error { target, .. } => target.clone(),
            Self::Disconnected => None,
        }
    }

    fn is_connected_to(&self, room: &VoiceRoomTarget) -> bool {
        matches!(
            self,
            Self::Connected { target, .. }
                if target.server_id == room.server_id && target.room_id == room.room_id
        )
    }

    fn is_connecting_to(&self, room: &VoiceRoomTarget) -> bool {
        matches!(
            self,
            Self::Connecting { target }
                if target.server_id == room.server_id && target.room_id == room.room_id
        )
    }
}

fn ensure_current_user_present(participants: &mut Vec<VoiceRoomParticipant>, user: &AuthUser) {
    if participants
        .iter()
        .any(|participant| participant.user_id == user.id)
    {
        return;
    }

    participants.push(VoiceRoomParticipant {
        user_id: user.id.clone(),
        nickname: user.nickname.clone(),
        avatar_url: user.avatar_url.clone(),
        joined_at: String::new(),
    });
}

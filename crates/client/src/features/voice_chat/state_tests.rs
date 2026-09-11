//! Проверки перехода состояний голосового подключения по снимкам участников.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use cheenhub_contracts::realtime::{VoiceRoomParticipant, VoiceRoomSnapshot};
use cheenhub_contracts::rest::AuthUser;
use dioxus::prelude::*;

use crate::features::microphone::MicrophoneHandle;
use crate::features::realtime::{RealtimeError, create_handle};

use super::actions::JoinedVoiceRoom;
use super::{VoiceConnectionHandle, VoiceConnectionParts, VoiceConnectionState, VoiceRoomTarget};

const CURRENT_USER_ID: &str = "current-user";
const PEER_USER_ID: &str = "peer-user";

fn current_user() -> AuthUser {
    AuthUser {
        id: CURRENT_USER_ID.to_owned(),
        nickname: "Текущий".to_owned(),
        email: "current@example.com".to_owned(),
        registered_at: "2026-01-01T00:00:00Z".to_owned(),
        has_password: false,
        avatar_url: None,
    }
}

fn participant(user_id: &str) -> VoiceRoomParticipant {
    VoiceRoomParticipant {
        user_id: user_id.to_owned(),
        nickname: user_id.to_owned(),
        avatar_url: None,
        joined_at: "2026-01-01T00:00:00Z".to_owned(),
    }
}

fn snapshot(
    target: &VoiceRoomTarget,
    participants: Vec<VoiceRoomParticipant>,
) -> VoiceRoomSnapshot {
    VoiceRoomSnapshot {
        server_id: target.server_id.clone(),
        room_id: target.room_id.clone(),
        participants,
        audio_bitrate_bps: None,
    }
}

/// Поколение join-операции, считающееся текущей в тестовых handle.
const ACTIVE_JOIN_GENERATION: u64 = 1;

fn voice_handle(state: VoiceConnectionState) -> (VoiceConnectionHandle, MicrophoneHandle) {
    voice_handle_with_generation(state, Rc::new(Cell::new(ACTIVE_JOIN_GENERATION)))
}

fn voice_handle_with_generation(
    state: VoiceConnectionState,
    join_generation: Rc<Cell<u64>>,
) -> (VoiceConnectionHandle, MicrophoneHandle) {
    let microphone = MicrophoneHandle::for_test();
    let handle = VoiceConnectionHandle::new(VoiceConnectionParts {
        state: Signal::new(state),
        kicked_from_room: Signal::new(None),
        speaking_users: Signal::new(Vec::new()),
        room_snapshots: Signal::new(Vec::new()),
        speaking_generations: Rc::new(RefCell::new(HashMap::new())),
        join_generation,
        realtime: create_handle(),
        microphone: microphone.clone(),
        current_user: current_user(),
    });
    (handle, microphone)
}

fn connected_participant_ids(state: &VoiceConnectionState) -> Option<Vec<String>> {
    match state {
        VoiceConnectionState::Connected { participants, .. } => Some(
            participants
                .iter()
                .map(|participant| participant.user_id.clone())
                .collect(),
        ),
        _ => None,
    }
}

fn with_dioxus_runtime<T>(body: impl FnOnce() -> T) -> T {
    let vdom = VirtualDom::new(|| rsx! {});
    vdom.in_scope(ScopeId::ROOT, body)
}

#[test]
fn participants_changed_confirms_join_while_connecting() {
    with_dioxus_runtime(|| {
        let target =
            VoiceRoomTarget::direct_message("conversation-1".to_owned(), "Лиса".to_owned());
        let (handle, _microphone) = voice_handle(VoiceConnectionState::Connecting {
            target: target.clone(),
        });

        handle.apply_snapshot(snapshot(
            &target,
            vec![participant(CURRENT_USER_ID), participant(PEER_USER_ID)],
        ));

        let participants = connected_participant_ids(&handle.state())
            .expect("participants changed event should confirm the in-flight join");
        assert_eq!(
            participants,
            vec![CURRENT_USER_ID.to_owned(), PEER_USER_ID.to_owned()]
        );
    });
}

#[test]
fn late_join_response_does_not_overwrite_fresher_participants_changed() {
    with_dioxus_runtime(|| {
        let target =
            VoiceRoomTarget::direct_message("conversation-1".to_owned(), "Лиса".to_owned());
        let (handle, _microphone) = voice_handle(VoiceConnectionState::Connecting {
            target: target.clone(),
        });

        handle.apply_snapshot(snapshot(
            &target,
            vec![participant(CURRENT_USER_ID), participant(PEER_USER_ID)],
        ));
        handle.complete_join(
            &target,
            ACTIVE_JOIN_GENERATION,
            JoinedVoiceRoom {
                snapshot: snapshot(&target, vec![participant(CURRENT_USER_ID)]),
                audio_bitrate_bps: 48_000,
            },
        );

        let participants = connected_participant_ids(&handle.state())
            .expect("state should stay connected after the stale join response");
        assert_eq!(
            participants,
            vec![CURRENT_USER_ID.to_owned(), PEER_USER_ID.to_owned()]
        );
    });
}

#[test]
fn late_join_response_applies_server_bitrate_without_touching_participants() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server(
            "server-1".to_owned(),
            "room-1".to_owned(),
            "Общий".to_owned(),
        );
        let (handle, microphone) = voice_handle(VoiceConnectionState::Connecting {
            target: target.clone(),
        });

        handle.apply_snapshot(snapshot(
            &target,
            vec![participant(CURRENT_USER_ID), participant(PEER_USER_ID)],
        ));
        assert!(connected_participant_ids(&handle.state()).is_some());

        handle.complete_join(
            &target,
            ACTIVE_JOIN_GENERATION,
            JoinedVoiceRoom {
                snapshot: snapshot(&target, vec![participant(CURRENT_USER_ID)]),
                audio_bitrate_bps: 48_000,
            },
        );

        let participants = connected_participant_ids(&handle.state())
            .expect("state should stay connected after the late join response");
        assert_eq!(
            participants,
            vec![CURRENT_USER_ID.to_owned(), PEER_USER_ID.to_owned()]
        );
        assert_eq!(microphone.target_bitrate_bps_for_test(), 48_000);
    });
}

#[test]
fn successful_join_response_completes_connecting_state() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server(
            "server-1".to_owned(),
            "room-1".to_owned(),
            "Общий".to_owned(),
        );
        let (handle, microphone) = voice_handle(VoiceConnectionState::Connecting {
            target: target.clone(),
        });

        handle.complete_join(
            &target,
            ACTIVE_JOIN_GENERATION,
            JoinedVoiceRoom {
                snapshot: snapshot(&target, vec![participant(CURRENT_USER_ID)]),
                audio_bitrate_bps: 48_000,
            },
        );

        let participants = connected_participant_ids(&handle.state())
            .expect("successful join response should complete the in-flight join");
        assert_eq!(participants, vec![CURRENT_USER_ID.to_owned()]);
        assert_eq!(microphone.target_bitrate_bps_for_test(), 48_000);
    });
}

#[test]
fn join_response_for_other_target_is_fully_ignored() {
    with_dioxus_runtime(|| {
        let current_target = VoiceRoomTarget::server(
            "server-1".to_owned(),
            "room-2".to_owned(),
            "Другая".to_owned(),
        );
        let stale_target = VoiceRoomTarget::server(
            "server-1".to_owned(),
            "room-1".to_owned(),
            "Общий".to_owned(),
        );
        let (handle, microphone) = voice_handle(VoiceConnectionState::Connecting {
            target: current_target.clone(),
        });

        handle.complete_join(
            &stale_target,
            ACTIVE_JOIN_GENERATION,
            JoinedVoiceRoom {
                snapshot: snapshot(&stale_target, vec![participant(CURRENT_USER_ID)]),
                audio_bitrate_bps: 48_000,
            },
        );

        assert!(matches!(
            handle.state(),
            VoiceConnectionState::Connecting { .. }
        ));
        assert_eq!(
            microphone.target_bitrate_bps_for_test(),
            cheenhub_contracts::media::VOICE_AUDIO_BITRATE_BPS
        );
    });
}

#[test]
fn stale_join_generation_is_fully_ignored_even_for_same_room() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server(
            "server-1".to_owned(),
            "room-1".to_owned(),
            "Общий".to_owned(),
        );
        // Текущей операцией считается rejoin той же комнаты (поколение 2);
        // ответ приходит от предыдущего join (поколение 1).
        let (handle, microphone) = voice_handle_with_generation(
            VoiceConnectionState::Connected {
                target: target.clone(),
                participants: vec![participant(CURRENT_USER_ID), participant(PEER_USER_ID)],
            },
            Rc::new(Cell::new(2)),
        );
        microphone.set_bitrate_bps(128_000);

        handle.complete_join(
            &target,
            1,
            JoinedVoiceRoom {
                snapshot: snapshot(&target, vec![participant(CURRENT_USER_ID)]),
                audio_bitrate_bps: 48_000,
            },
        );

        let participants = connected_participant_ids(&handle.state())
            .expect("state should stay connected after the stale generation response");
        assert_eq!(
            participants,
            vec![CURRENT_USER_ID.to_owned(), PEER_USER_ID.to_owned()]
        );
        assert_eq!(microphone.target_bitrate_bps_for_test(), 128_000);
    });
}

#[test]
fn current_join_generation_applies_bitrate_to_connected_room() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server(
            "server-1".to_owned(),
            "room-1".to_owned(),
            "Общий".to_owned(),
        );
        let (handle, microphone) = voice_handle(VoiceConnectionState::Connected {
            target: target.clone(),
            participants: vec![participant(CURRENT_USER_ID), participant(PEER_USER_ID)],
        });

        handle.complete_join(
            &target,
            ACTIVE_JOIN_GENERATION,
            JoinedVoiceRoom {
                snapshot: snapshot(&target, vec![participant(CURRENT_USER_ID)]),
                audio_bitrate_bps: 48_000,
            },
        );

        let participants = connected_participant_ids(&handle.state())
            .expect("state should stay connected after the current generation response");
        assert_eq!(
            participants,
            vec![CURRENT_USER_ID.to_owned(), PEER_USER_ID.to_owned()]
        );
        assert_eq!(microphone.target_bitrate_bps_for_test(), 48_000);
    });
}

#[test]
fn stale_join_generation_failure_is_ignored_for_same_room() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server(
            "server-1".to_owned(),
            "room-1".to_owned(),
            "Общий".to_owned(),
        );
        // Текущей операцией считается rejoin той же комнаты (поколение 2).
        let (handle, _microphone) = voice_handle_with_generation(
            VoiceConnectionState::Connecting {
                target: target.clone(),
            },
            Rc::new(Cell::new(2)),
        );

        handle.apply_join_failure(&target, 1, RealtimeError::new("старая ошибка входа"));

        assert!(matches!(
            handle.state(),
            VoiceConnectionState::Connecting { .. }
        ));
    });
}

#[test]
fn current_join_generation_failure_sets_error() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server(
            "server-1".to_owned(),
            "room-1".to_owned(),
            "Общий".to_owned(),
        );
        let (handle, _microphone) = voice_handle(VoiceConnectionState::Connecting {
            target: target.clone(),
        });

        handle.apply_join_failure(
            &target,
            ACTIVE_JOIN_GENERATION,
            RealtimeError::new("ошибка входа"),
        );

        assert!(matches!(handle.state(), VoiceConnectionState::Error { .. }));
    });
}

#[test]
fn stale_join_generation_timeout_is_ignored_for_same_room() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server(
            "server-1".to_owned(),
            "room-1".to_owned(),
            "Общий".to_owned(),
        );
        // Текущей операцией считается rejoin той же комнаты (поколение 2).
        let (handle, _microphone) = voice_handle_with_generation(
            VoiceConnectionState::Connecting {
                target: target.clone(),
            },
            Rc::new(Cell::new(2)),
        );

        handle.apply_join_timeout(&target, 1);

        assert!(matches!(
            handle.state(),
            VoiceConnectionState::Connecting { .. }
        ));
    });
}

#[test]
fn current_join_generation_timeout_sets_error() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server(
            "server-1".to_owned(),
            "room-1".to_owned(),
            "Общий".to_owned(),
        );
        let (handle, _microphone) = voice_handle(VoiceConnectionState::Connecting {
            target: target.clone(),
        });

        handle.apply_join_timeout(&target, ACTIVE_JOIN_GENERATION);

        assert!(matches!(handle.state(), VoiceConnectionState::Error { .. }));
    });
}

#[test]
fn participants_changed_without_current_user_keeps_connecting() {
    with_dioxus_runtime(|| {
        let target =
            VoiceRoomTarget::direct_message("conversation-1".to_owned(), "Лиса".to_owned());
        let (handle, _microphone) = voice_handle(VoiceConnectionState::Connecting {
            target: target.clone(),
        });

        handle.apply_snapshot(snapshot(&target, vec![participant(PEER_USER_ID)]));

        assert!(matches!(
            handle.state(),
            VoiceConnectionState::Connecting { .. }
        ));
    });
}

#[test]
fn connected_server_room_updates_participants_from_snapshot() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server(
            "server-1".to_owned(),
            "room-1".to_owned(),
            "Общий".to_owned(),
        );
        let (handle, _microphone) = voice_handle(VoiceConnectionState::Connected {
            target: target.clone(),
            participants: vec![participant(CURRENT_USER_ID)],
        });

        handle.apply_snapshot(snapshot(
            &target,
            vec![participant(CURRENT_USER_ID), participant(PEER_USER_ID)],
        ));

        let participants = connected_participant_ids(&handle.state())
            .expect("connected server room should stay connected");
        assert_eq!(
            participants,
            vec![CURRENT_USER_ID.to_owned(), PEER_USER_ID.to_owned()]
        );
    });
}

#[test]
fn connected_server_room_without_current_user_marks_kick() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server(
            "server-1".to_owned(),
            "room-1".to_owned(),
            "Общий".to_owned(),
        );
        let (handle, _microphone) = voice_handle(VoiceConnectionState::Connected {
            target: target.clone(),
            participants: vec![participant(CURRENT_USER_ID), participant(PEER_USER_ID)],
        });

        handle.apply_snapshot(snapshot(&target, vec![participant(PEER_USER_ID)]));

        assert!(matches!(handle.state(), VoiceConnectionState::Disconnected));
        assert_eq!((handle.kicked_from_room)(), Some("Общий".to_owned()));
    });
}

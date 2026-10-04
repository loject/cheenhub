//! Регрессии изоляции принятия личного звонка между устройствами.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use super::super::state::{VoiceConnectionParts, VoiceConnectionState};
use super::*;
use crate::features::microphone::MicrophoneHandle;
use crate::features::realtime::create_handle;
use cheenhub_contracts::rest::AuthUser;

const CURRENT_USER_ID: &str = "current-user";

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
        remembered_room: Signal::new(None),
        speaking_generations: Rc::new(RefCell::new(HashMap::new())),
        join_generation,
        realtime: create_handle(),
        microphone: microphone.clone(),
        current_user: current_user(),
    });
    (handle, microphone)
}

fn with_runtime(body: impl FnOnce()) {
    let vdom = VirtualDom::new(|| rsx! {});
    vdom.in_scope(ScopeId::ROOT, body);
}

fn handle() -> DirectCallHandle {
    let (voice, _) = voice_handle(VoiceConnectionState::Disconnected);
    DirectCallHandle::new(
        Signal::new(DirectCallUiState::Idle),
        Signal::new(false),
        Signal::new(None),
        create_handle(),
        voice,
        CURRENT_USER_ID.to_owned(),
    )
}

fn call(state: DirectCallState) -> DirectCallSnapshot {
    DirectCallSnapshot {
        call_id: "call-1".to_owned(),
        conversation_id: "conversation-1".to_owned(),
        caller_user_id: "peer-user".to_owned(),
        caller_nickname: "Лиса".to_owned(),
        caller_avatar_url: None,
        callee_user_id: CURRENT_USER_ID.to_owned(),
        callee_nickname: "Текущий".to_owned(),
        callee_avatar_url: None,
        state,
        started_at: "2026-10-04T00:00:00Z".to_owned(),
        answered_at: (state == DirectCallState::Active).then(|| "2026-10-04T00:00:01Z".to_owned()),
        ended_at: None,
        end_reason: None,
    }
}

#[test]
fn accepting_on_another_device_dismisses_incoming_call_without_joining_media() {
    with_runtime(|| {
        let handle = handle();
        handle.apply_snapshot(call(DirectCallState::Ringing));
        assert!(handle.incoming_call().is_some());

        handle.apply_event(DirectCallLifecycleEvent {
            recipient_user_id: CURRENT_USER_ID.to_owned(),
            call: call(DirectCallState::Active),
        });

        assert_eq!(handle.voice.state(), VoiceConnectionState::Disconnected);
        assert_eq!(handle.state(), DirectCallUiState::Idle);
        assert!(handle.incoming_call().is_none());
    });
}

#[test]
fn recovering_active_call_on_another_device_does_not_join_media() {
    with_runtime(|| {
        let handle = handle();

        handle.apply_snapshot(call(DirectCallState::Active));

        assert_eq!(handle.voice.state(), VoiceConnectionState::Disconnected);
        assert_eq!(handle.state(), DirectCallUiState::Idle);
    });
}

#[test]
fn late_accept_failure_does_not_restore_dismissed_incoming_call() {
    with_runtime(|| {
        let handle = handle();
        let ringing = call(DirectCallState::Ringing);
        handle.apply_snapshot(ringing.clone());
        let mut state = handle.state;
        state.set(DirectCallUiState::Idle);

        handle.apply_action_error(ringing, "already accepted".to_owned(), "respond");

        assert_eq!(handle.state(), DirectCallUiState::Idle);
    });
}

#[test]
fn successful_local_accept_joins_after_broadcast_arrives_before_response() {
    with_runtime(|| {
        let handle = handle();
        handle.apply_snapshot(call(DirectCallState::Ringing));
        handle.apply_event(DirectCallLifecycleEvent {
            recipient_user_id: CURRENT_USER_ID.to_owned(),
            call: call(DirectCallState::Active),
        });

        handle.apply_local_snapshot(call(DirectCallState::Active));

        assert!(
            matches!(handle.state(), DirectCallUiState::Call(call) if call.state == DirectCallState::Active)
        );
        assert!(
            matches!(handle.voice.state(), VoiceConnectionState::Connecting { target } if target.room_id == "conversation-1")
        );
    });
}

#[test]
fn only_device_that_started_outgoing_call_joins_when_peer_accepts() {
    with_runtime(|| {
        let origin = handle();
        let other = handle();
        let mut ringing = call(DirectCallState::Ringing);
        ringing.caller_user_id = CURRENT_USER_ID.to_owned();
        ringing.callee_user_id = "peer-user".to_owned();
        origin.apply_local_snapshot(ringing.clone());
        other.apply_snapshot(ringing.clone());
        let mut active = ringing;
        active.state = DirectCallState::Active;
        active.answered_at = Some("2026-10-04T00:00:01Z".to_owned());

        for device in [&origin, &other] {
            device.apply_event(DirectCallLifecycleEvent {
                recipient_user_id: CURRENT_USER_ID.to_owned(),
                call: active.clone(),
            });
        }

        assert!(matches!(
            origin.voice.state(),
            VoiceConnectionState::Connecting { .. }
        ));
        assert_eq!(other.voice.state(), VoiceConnectionState::Disconnected);
        assert_eq!(other.state(), DirectCallUiState::Idle);
    });
}

#[test]
fn owning_device_can_recover_active_call_after_transport_disconnect() {
    with_runtime(|| {
        let handle = handle();
        handle.apply_local_snapshot(call(DirectCallState::Active));
        let mut voice_state = handle.voice.state;
        voice_state.set(VoiceConnectionState::Disconnected);

        handle.apply_snapshot(call(DirectCallState::Active));

        assert!(matches!(
            handle.voice.state(),
            VoiceConnectionState::Connecting { .. }
        ));
    });
}

#[test]
fn ending_call_clears_local_ownership() {
    with_runtime(|| {
        let handle = handle();
        handle.apply_local_snapshot(call(DirectCallState::Active));

        handle.apply_snapshot(call(DirectCallState::Ended));
        let mut voice_state = handle.voice.state;
        voice_state.set(VoiceConnectionState::Disconnected);
        handle.apply_snapshot(call(DirectCallState::Active));

        assert_eq!(handle.voice.state(), VoiceConnectionState::Disconnected);
        assert_eq!(handle.state(), DirectCallUiState::Idle);
    });
}

#[test]
fn late_accept_failure_after_other_device_accepts_does_not_ring_again() {
    with_runtime(|| {
        let handle = handle();
        let ringing = call(DirectCallState::Ringing);
        handle.apply_snapshot(ringing.clone());
        handle.apply_snapshot(call(DirectCallState::Active));

        handle.apply_action_error(ringing, "already accepted".to_owned(), "respond");

        assert_eq!(handle.state(), DirectCallUiState::Idle);
        assert!(handle.incoming_call().is_none());
    });
}

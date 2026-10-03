//! Проверки правила восстановления голоса после переподключения.

use super::should_rejoin;
use crate::features::voice_chat::state::VoiceRoomTarget;

#[test]
fn reconnect_rejoins_remembered_server_room() {
    let target = VoiceRoomTarget::server(
        "server-1".to_owned(),
        "room-1".to_owned(),
        "Общая".to_owned(),
    );

    assert!(should_rejoin(Some(&target), false));
}

#[test]
fn reconnect_skips_join_when_room_is_already_active() {
    let target = VoiceRoomTarget::server(
        "server-1".to_owned(),
        "room-1".to_owned(),
        "Общая".to_owned(),
    );

    assert!(!should_rejoin(Some(&target), true));
}

#[test]
fn reconnect_skips_direct_message_room_owned_by_call_module() {
    let target = VoiceRoomTarget::direct_message("conversation-1".to_owned(), "Лиса".to_owned());

    assert!(!should_rejoin(Some(&target), false));
}

#[test]
fn reconnect_without_remembered_room_does_nothing() {
    assert!(!should_rejoin(None, false));
}

#[test]
fn reconnect_consumes_remembered_target_before_join() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server("server-1".into(), "room-1".into(), "Общая".into());
        let (mut handle, _) = voice_handle(VoiceConnectionState::Disconnected);
        handle.remember_room(target.clone());

        super::rejoin_remembered_room(&handle);

        assert!(
            matches!(handle.state(), VoiceConnectionState::Connecting { target: actual } if actual == target)
        );
        assert_eq!(handle.take_remembered_room(), None);
    });
}

#[test]
fn reconnect_preserves_newly_active_target() {
    with_dioxus_runtime(|| {
        let previous = VoiceRoomTarget::server("server-1".into(), "room-1".into(), "Общая".into());
        let current = VoiceRoomTarget::server("server-1".into(), "room-2".into(), "Новая".into());
        let (mut handle, _) = voice_handle(VoiceConnectionState::Connecting {
            target: current.clone(),
        });
        handle.remember_room(previous);

        super::rejoin_remembered_room(&handle);

        assert!(
            matches!(handle.state(), VoiceConnectionState::Connecting { target: actual } if actual == current)
        );
        assert_eq!(handle.take_remembered_room(), None);
    });
}

use crate::features::microphone::MicrophoneHandle;
use crate::features::realtime::create_handle;
use crate::features::voice_chat::state::{
    VoiceConnectionHandle, VoiceConnectionParts, VoiceConnectionState,
};
use cheenhub_contracts::rest::AuthUser;
use dioxus::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
const CURRENT_USER_ID: &str = "current-user";
const ACTIVE_JOIN_GENERATION: u64 = 1;
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

fn with_dioxus_runtime<T>(body: impl FnOnce() -> T) -> T {
    let vdom = VirtualDom::new(|| rsx! {});
    vdom.in_scope(ScopeId::ROOT, body)
}

#[test]
fn transport_disconnect_during_manual_leave_does_not_restore_departing_room() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server("server-1".into(), "room-1".into(), "Общая".into());
        let (mut handle, _) = voice_handle(VoiceConnectionState::Connected {
            target: target.clone(),
            participants: vec![],
        });

        handle.leave();
        super::remember_active_room(&mut handle);
        handle.state.set(VoiceConnectionState::Disconnected);
        super::rejoin_remembered_room(&handle);

        assert_eq!(handle.state(), VoiceConnectionState::Disconnected);
        assert_eq!(handle.take_remembered_room(), None);
    });
}

#[test]
fn transport_disconnect_after_failed_join_does_not_retry_failed_target() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server("server-1".into(), "room-1".into(), "Общая".into());
        let (mut handle, _) = voice_handle(VoiceConnectionState::Error {
            target: Some(target),
            message: "Вход отклонён".into(),
        });

        super::remember_active_room(&mut handle);
        handle.state.set(VoiceConnectionState::Disconnected);
        super::rejoin_remembered_room(&handle);

        assert_eq!(handle.state(), VoiceConnectionState::Disconnected);
        assert_eq!(handle.take_remembered_room(), None);
    });
}

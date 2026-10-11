use super::*;
use ashpd::zbus::{
    Message,
    zvariant::{ObjectPath, OwnedValue},
};
use std::collections::HashMap;

const SESSION: &str = "/org/freedesktop/portal/desktop/session/cheenhub/one";

fn message(member: &str, session: &str, shortcut: &str) -> Message {
    Message::signal(
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.GlobalShortcuts",
        member,
    )
    .unwrap()
    .build(&(
        ObjectPath::try_from(session).unwrap(),
        shortcut,
        100_u64,
        HashMap::<String, OwnedValue>::new(),
    ))
    .unwrap()
}

#[test]
fn queued_activation_and_release_leave_gate_closed_in_bus_order() {
    let state = super::super::super::state::State::default();
    state.recover();
    let signals = [
        message("Activated", SESSION, super::super::SHORTCUT_ID),
        message("Deactivated", SESSION, super::super::SHORTCUT_ID),
    ];

    for signal in signals {
        let Event::Held(held) = decode(&signal, SESSION).unwrap() else {
            panic!("hold event");
        };
        state.event(held);
    }

    assert_eq!(
        state.held_epoch(&std::sync::atomic::AtomicU64::new(0)),
        None
    );
}

#[test]
fn other_session_and_other_shortcut_cannot_open_gate() {
    assert!(matches!(
        decode(
            &message("Activated", "/another/session", super::super::SHORTCUT_ID),
            SESSION
        )
        .unwrap(),
        Event::Ignored
    ));
    assert!(matches!(
        decode(&message("Activated", SESSION, "another_shortcut"), SESSION).unwrap(),
        Event::Ignored
    ));
}

#[test]
fn malformed_release_is_an_error_instead_of_an_ignored_signal() {
    let signal = Message::signal(
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.GlobalShortcuts",
        "Deactivated",
    )
    .unwrap()
    .build(&"invalid")
    .unwrap();

    assert!(decode(&signal, SESSION).is_err());
}

#[test]
fn pending_assignment_removal_is_not_discarded_with_old_holds() {
    let mut properties = HashMap::<String, OwnedValue>::new();
    properties.insert(
        "description".into(),
        ashpd::zbus::zvariant::Str::from("Push-to-talk").into(),
    );
    properties.insert(
        "trigger_description".into(),
        ashpd::zbus::zvariant::Str::from("Ctrl + Space").into(),
    );
    let change = Message::signal(
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.GlobalShortcuts",
        "ShortcutsChanged",
    )
    .unwrap()
    .build(&(
        ObjectPath::try_from(SESSION).unwrap(),
        vec![(super::super::SHORTCUT_ID, properties)],
    ))
    .unwrap();
    let removal = Message::signal(
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.GlobalShortcuts",
        "ShortcutsChanged",
    )
    .unwrap()
    .build(&(
        ObjectPath::try_from(SESSION).unwrap(),
        Vec::<(String, HashMap<String, OwnedValue>)>::new(),
    ))
    .unwrap();
    let mut label = None;
    fold_pending(&change, SESSION, &mut label).unwrap();
    assert_eq!(label.as_deref(), Some("Ctrl + Space"));
    fold_pending(
        &message("Activated", SESSION, super::super::SHORTCUT_ID),
        SESSION,
        &mut label,
    )
    .unwrap();

    let result = fold_pending(&removal, SESSION, &mut label);

    assert!(result.is_err());
    assert_eq!(label, None);
}

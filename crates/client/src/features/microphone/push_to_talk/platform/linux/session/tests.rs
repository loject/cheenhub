use super::*;

fn service(users: usize) -> Service {
    let (commands, _) = mpsc::unbounded_channel();
    Service {
        state: super::super::state::State::default(),
        available: std::sync::atomic::AtomicBool::new(true),
        users: std::sync::atomic::AtomicUsize::new(users),
        commands,
        stopped: std::sync::atomic::AtomicBool::new(false),
        updates: tokio::sync::watch::channel(super::super::updates::initial()).0,
    }
}

#[test]
fn external_binding_change_publishes_metadata_and_invalidates_old_audio() {
    let _guard = binding::TEST_LOCK.lock().unwrap();
    let service = service(1);
    let mut listener = service.updates.subscribe();
    let mut assignment = Assignment::new(None);
    let recording = std::sync::atomic::AtomicU64::new(0);
    ready(&service);
    service.state.event(true);
    let old_epoch = service.state.held_epoch(&recording).unwrap();

    let end = changed(&service, &mut assignment, Some("Ctrl + Space"));

    assert!(end.is_none());
    assert!(listener.has_changed().unwrap());
    let update = listener.borrow_and_update().clone();
    assert_eq!(update.status, Status::Ready);
    assert_eq!(
        update
            .binding
            .unwrap()
            .storage_value()
            .splitn(3, ':')
            .last(),
        Some("Ctrl + Space")
    );
    assert_eq!(service.state.held_epoch(&recording), None);
    service.state.event(true);
    assert_ne!(service.state.held_epoch(&recording).unwrap(), old_epoch);
}

#[test]
fn revoked_binding_publishes_failure_and_closes_gate() {
    let _guard = binding::TEST_LOCK.lock().unwrap();
    let service = service(1);
    let mut assignment = Assignment::new(None);
    ready(&service);
    service.state.event(true);

    let end = changed(&service, &mut assignment, None);

    assert!(matches!(end, Some(End::Failed)));
    assert!(matches!(service.updates.borrow().status, Status::Failed(_)));
    assert_eq!(
        service.updates.borrow().binding,
        Some(binding::PushToTalkKey::default())
    );
    assert_eq!(
        service
            .state
            .held_epoch(&std::sync::atomic::AtomicU64::new(0)),
        None
    );
}

#[test]
fn completed_assignment_without_consumers_requests_session_release() {
    let _guard = binding::TEST_LOCK.lock().unwrap();
    let service = service(0);
    let (reply, mut result) = oneshot::channel();
    let mut assignment = Assignment::new(Some(reply));
    service.configuring(true);

    let end = changed(&service, &mut assignment, Some("Alt + Space"));

    assert!(matches!(end, Some(End::Idle)));
    assert!(result.try_recv().unwrap().is_ok());
    assert!(!service.updates.borrow().configuring);
    assert_eq!(
        service
            .state
            .held_epoch(&std::sync::atomic::AtomicU64::new(0)),
        None
    );
}

fn queued_hold(path: &str) -> zbus::Message {
    zbus::Message::signal(
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.GlobalShortcuts",
        "Activated",
    )
    .unwrap()
    .build(&(
        zbus::zvariant::ObjectPath::try_from(path).unwrap(),
        SHORTCUT_ID,
        100_u64,
        std::collections::HashMap::<String, zbus::zvariant::OwnedValue>::new(),
    ))
    .unwrap()
}

#[test]
fn queued_activation_during_cancel_does_not_resume_old_hold() {
    let service = service(1);
    let mut label = Some("Ctrl + Space".into());
    let (reply, _result) = oneshot::channel();
    let mut assignment = Assignment::new(Some(reply));
    let queued = queued_hold("/session/one");
    ready(&service);
    service.state.event(true);

    assignment.cancel(&service);
    fold_pending(&service, &queued, "/session/one", &mut label).unwrap();
    ready(&service);

    assert_eq!(
        service
            .state
            .held_epoch(&std::sync::atomic::AtomicU64::new(0)),
        None
    );
    service.state.event(true);
    assert!(
        service
            .state
            .held_epoch(&std::sync::atomic::AtomicU64::new(0))
            .is_some()
    );
}

#[test]
fn queued_activation_from_previous_capture_cannot_open_replacement_gate() {
    let service = service(1);
    let mut label = Some("Ctrl + Space".into());
    let queued = queued_hold("/session/one");
    ready(&service);
    service.state.event(true);

    service.state.fail();
    fold_pending(&service, &queued, "/session/one", &mut label).unwrap();
    ready(&service);

    assert_eq!(
        service
            .state
            .held_epoch(&std::sync::atomic::AtomicU64::new(0)),
        None
    );
}

#[test]
fn queued_revocation_at_finish_publishes_binding_removal_before_error() {
    let _guard = binding::TEST_LOCK.lock().unwrap();
    let service = service(1);
    let mut label = Some("Ctrl + Space".into());
    let removed = zbus::Message::signal(
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.GlobalShortcuts",
        "ShortcutsChanged",
    )
    .unwrap()
    .build(&(
        zbus::zvariant::ObjectPath::try_from("/session/one").unwrap(),
        Vec::<(
            String,
            std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        )>::new(),
    ))
    .unwrap();

    let result = fold_pending(&service, &removed, "/session/one", &mut label);

    assert!(result.is_err());
    assert_eq!(label, None);
    assert_eq!(binding::raw_label(), "");
    assert_eq!(
        service.updates.borrow().binding,
        Some(binding::PushToTalkKey::default())
    );
    assert!(matches!(service.updates.borrow().status, Status::Failed(_)));
}

#[tokio::test]
async fn cancelling_assignment_notifies_session_owner() {
    let (reply, result) = oneshot::channel();
    let mut pending = Assignment::new(Some(reply));

    drop(result);

    tokio::time::timeout(Duration::from_millis(50), pending.cancelled())
        .await
        .expect("cancelled assignment must release the session");
}

#[tokio::test]
#[ignore = "Требуются D-Bus session bus и установленный GlobalShortcuts Portal v1 с Host Portal Registry"]
async fn live_portal_identity_session_opens_and_closes() {
    let (portal, session) = open().await.expect("identified portal session");

    session.close().await.expect("portal session release");

    assert!(portal.version() >= 1);
}

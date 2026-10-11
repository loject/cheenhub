use super::*;
use crate::features::microphone::push_to_talk::platform::linux::{state::State, updates};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize};
use tokio::sync::{mpsc, watch};

fn service() -> Service {
    let (commands, _) = mpsc::unbounded_channel();
    Service {
        state: State::default(),
        available: AtomicBool::new(true),
        users: AtomicUsize::new(0),
        commands,
        stopped: AtomicBool::new(false),
        updates: watch::channel(updates::initial()).0,
    }
}

#[tokio::test]
async fn completing_configuration_without_change_keeps_binding() {
    let service = service();
    let previous = PushToTalkKey::default();
    service
        .updates
        .send_modify(|update| update.binding = Some(previous));
    let (reply, result) = oneshot::channel();
    let mut assignment = Assignment::new(Some(reply));
    service.configuring(true);

    assignment.complete(&service);

    assert_eq!(result.await.unwrap().unwrap(), previous);
    assert!(!service.updates.borrow().configuring);
    assert!(!assignment.active());
}

#[tokio::test]
async fn ui_cancellation_closes_gate_without_deleting_binding() {
    let service = service();
    let previous = PushToTalkKey::default();
    service
        .updates
        .send_modify(|update| update.binding = Some(previous));
    service.state.recover();
    service.state.event(true);
    let (reply, result) = oneshot::channel();
    let mut assignment = Assignment::new(Some(reply));
    drop(result);

    tokio::time::timeout(std::time::Duration::from_millis(50), assignment.cancelled())
        .await
        .unwrap();
    assignment.cancel(&service);

    assert_eq!(service.state.held_epoch(&AtomicU64::new(0)), None);
    assert_eq!(service.updates.borrow().binding, Some(previous));
    assert!(!assignment.active());
}

#[tokio::test]
async fn version_one_reassignment_error_preserves_previous_binding() {
    let service = service();
    let previous = PushToTalkKey::default();
    service
        .updates
        .send_modify(|update| update.binding = Some(previous));
    let (reply, result) = oneshot::channel();
    let mut assignment = Assignment::new(Some(reply));

    assignment.reject(&service, "Откройте системные настройки");

    assert!(result.await.unwrap().is_err());
    assert_eq!(service.updates.borrow().binding, Some(previous));
}

#[test]
fn release_during_assignment_keeps_session_until_ui_finishes() {
    let service = service();
    let (reply, _result) = oneshot::channel();
    let mut assignment = Assignment::new(Some(reply));

    assert!(assignment.keeps_session(0));
    assignment.cancel(&service);

    assert!(!assignment.keeps_session(0));
    assert!(assignment.keeps_session(1));
}

#[test]
fn ensure_after_release_keeps_completed_assignment_session_for_new_consumer() {
    let service = service();
    let (reply, _result) = oneshot::channel();
    let mut assignment = Assignment::new(Some(reply));

    assignment.complete(&service);

    assert!(assignment.keeps_session(1));
    assert!(!assignment.keeps_session(0));
}

//! Проверки in-memory хранилища личных звонков.

use chrono::{Duration, Utc};

use super::{DirectCall, DirectCallStoreError, InMemoryDirectCallStore};

fn call(caller_user_id: uuid::Uuid, callee_user_id: uuid::Uuid) -> DirectCall {
    let now = Utc::now();
    DirectCall {
        id: uuid::Uuid::new_v4(),
        conversation_id: uuid::Uuid::new_v4(),
        caller_user_id,
        caller_nickname: "caller".to_owned(),
        caller_avatar_url: None,
        callee_user_id,
        callee_nickname: "callee".to_owned(),
        callee_avatar_url: None,
        started_at: now,
        expires_at: now + Duration::seconds(45),
        answered_at: None,
        callee_notified: true,
    }
}

#[tokio::test]
async fn busy_caller_cannot_start_second_call() {
    let store = InMemoryDirectCallStore::default();
    let caller = uuid::Uuid::new_v4();
    let callee = uuid::Uuid::new_v4();
    store
        .start(call(caller, callee))
        .await
        .expect("first call should start");

    let error = store
        .start(call(caller, uuid::Uuid::new_v4()))
        .await
        .expect_err("busy caller should reject another call");

    assert_eq!(error, DirectCallStoreError::CallerBusy);
}

#[tokio::test]
async fn busy_callee_does_not_reject_or_receive_second_call() {
    let store = InMemoryDirectCallStore::default();
    let first_caller = uuid::Uuid::new_v4();
    let second_caller = uuid::Uuid::new_v4();
    let callee = uuid::Uuid::new_v4();
    store
        .start(call(first_caller, callee))
        .await
        .expect("first call should start");

    let second = store
        .start(call(second_caller, callee))
        .await
        .expect("second caller should see a ringing call");

    assert!(!second.callee_notified);
    assert_eq!(store.list_for_user(&second_caller).await, vec![second]);
    assert_eq!(store.list_for_user(&callee).await.len(), 1);
}

#[tokio::test]
async fn pending_call_expires() {
    let store = InMemoryDirectCallStore::default();
    let mut pending = call(uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    pending.expires_at = Utc::now() - Duration::seconds(1);
    store
        .start(pending.clone())
        .await
        .expect("call should start");

    assert_eq!(store.expire(&pending.id, Utc::now()).await, Some(pending));
}

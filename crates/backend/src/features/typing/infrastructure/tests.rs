//! Проверки оперативного хранилища состояния набора.

use tokio::time::Instant;
use uuid::Uuid;

use super::{InMemoryTypingStore, TypingAuthorEntry, TypingTarget};

fn entry(
    target: TypingTarget,
    user_id: Uuid,
    realtime_stream_id: Uuid,
    nickname: &str,
) -> TypingAuthorEntry {
    TypingAuthorEntry {
        target,
        user_id,
        nickname: nickname.to_owned(),
        avatar_url: None,
        realtime_stream_id,
        refreshed_at: Instant::now(),
    }
}

#[tokio::test]
async fn first_start_reports_change_and_repeat_refresh_does_not() {
    let store = InMemoryTypingStore::default();
    let target = TypingTarget::room(Uuid::new_v4(), Uuid::new_v4());
    let stream_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    assert!(
        store
            .start(entry(target, user_id, stream_id, "пишет"))
            .await,
        "первый старт набора должен рассылаться участникам"
    );
    assert!(
        !store
            .start(entry(target, user_id, stream_id, "пишет"))
            .await,
        "продление уже известного набора не должно рассылаться повторно"
    );
    assert_eq!(store.typers(target).await.len(), 1);
}

#[tokio::test]
async fn different_users_type_independently() {
    let store = InMemoryTypingStore::default();
    let target = TypingTarget::room(Uuid::new_v4(), Uuid::new_v4());
    let stream_id = Uuid::new_v4();

    assert!(
        store
            .start(entry(target, Uuid::new_v4(), stream_id, "первый"))
            .await
    );
    assert!(
        store
            .start(entry(target, Uuid::new_v4(), stream_id, "второй"))
            .await
    );

    assert_eq!(store.typers(target).await.len(), 2);
    assert!(
        store
            .typers(TypingTarget::room(Uuid::new_v4(), Uuid::new_v4()))
            .await
            .is_empty(),
        "набор другой комнаты не должен попадать в снимок"
    );
}

#[tokio::test]
async fn stop_only_clears_own_stream_state() {
    let store = InMemoryTypingStore::default();
    let target = TypingTarget::direct_message(Uuid::new_v4());
    let user_id = Uuid::new_v4();
    let owner_stream = Uuid::new_v4();
    let foreign_stream = Uuid::new_v4();

    store
        .start(entry(target, user_id, owner_stream, "пишет"))
        .await;

    assert!(
        store.stop(target, user_id, foreign_stream).await.is_none(),
        "чужой поток не должен снимать чужое состояние набора"
    );
    assert!(
        store.stop(target, user_id, owner_stream).await.is_some(),
        "владелец набора должен снимать его сам"
    );
    assert!(store.typers(target).await.is_empty());
}

#[tokio::test]
async fn closed_stream_removes_its_entries() {
    let store = InMemoryTypingStore::default();
    let target = TypingTarget::room(Uuid::new_v4(), Uuid::new_v4());
    let stream_id = Uuid::new_v4();

    store
        .start(entry(target, Uuid::new_v4(), stream_id, "пишет"))
        .await;

    assert_eq!(store.remove_stream(stream_id).await.len(), 1);
    assert!(
        store.typers(target).await.is_empty(),
        "после закрытия потока набор не должен оставаться в снимке"
    );
}

#[tokio::test]
async fn expired_entry_disappears_from_snapshot() {
    let store = InMemoryTypingStore::default();
    let target = TypingTarget::room(Uuid::new_v4(), Uuid::new_v4());
    let mut stale = entry(target, Uuid::new_v4(), Uuid::new_v4(), "пишет");
    stale.refreshed_at = Instant::now() - super::TYPING_TTL;

    store.entries.lock().await.push(stale);

    assert!(
        store.typers(target).await.is_empty(),
        "запись без продления дольше времени жизни должна истечь"
    );
}

#[tokio::test]
async fn remove_expired_reports_entries_for_cancellation() {
    // Регрессия: истекшие записи исчезали из памяти молча, и получатели события
    // об отмене не получали, поэтому индикатор «печатает…» оставался висеть.
    let store = InMemoryTypingStore::default();
    let target = TypingTarget::room(Uuid::new_v4(), Uuid::new_v4());
    let mut stale = entry(target, Uuid::new_v4(), Uuid::new_v4(), "пишет");
    stale.refreshed_at = Instant::now() - super::TYPING_TTL;
    store.entries.lock().await.push(stale);

    let fresh = entry(target, Uuid::new_v4(), Uuid::new_v4(), "пишет");
    store.entries.lock().await.push(fresh.clone());

    let expired = store.remove_expired().await;

    assert_eq!(
        expired.len(),
        1,
        "фоновая проверка должна вернуть истекшую запись для рассылки отмены"
    );
    assert_eq!(
        store.typers(target).await.len(),
        1,
        "живая запись должна остаться в снимке"
    );
}

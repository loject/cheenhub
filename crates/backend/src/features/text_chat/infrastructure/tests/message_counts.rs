//! Проверки счётчиков сообщений для хранилища текстового чата.

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::super::{InMemoryTextChatStore, TextChatStore};
use crate::features::text_chat::domain::TextMessage;

/// Три сообщения: два в одной минуте, одно в соседней, одно вне окна.
async fn fill_messages(store: &dyn TextChatStore, now: DateTime<Utc>) {
    let minute = DateTime::from_timestamp(now.timestamp() - now.timestamp() % 60 - 120, 0)
        .expect("valid minute");
    for (server_id, room_id, author_user_id, created_at) in [
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), minute),
        (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            minute + Duration::seconds(20),
        ),
        (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            minute + Duration::minutes(1),
        ),
        (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            minute - Duration::hours(25),
        ),
    ] {
        store
            .insert_text_message(TextMessage {
                id: Uuid::new_v4(),
                server_id,
                room_id,
                author_user_id,
                author_nickname: "stats".to_owned(),
                body: "message".to_owned(),
                attachments: Vec::new(),
                created_at,
                deleted_at: None,
                deleted_by_user_id: None,
            })
            .await
            .expect("insert message");
    }
}

/// Окно суток используется и в общем счётчике, и в разбивке по минутам.
async fn assert_message_count_contract(store: &dyn TextChatStore) {
    let now = Utc::now();
    fill_messages(store, now).await;

    assert_eq!(
        store
            .count_text_messages()
            .await
            .expect("count all messages"),
        4,
        "общий счётчик учитывает сообщения за всё время"
    );

    let per_minute = store
        .count_messages_per_minute(now - Duration::hours(24), now)
        .await
        .expect("count messages per minute");
    assert_eq!(
        per_minute
            .iter()
            .map(|sample| sample.messages)
            .collect::<Vec<_>>(),
        vec![2, 1],
        "минуты с сообщениями отдаются в хронологическом порядке"
    );
    assert!(
        per_minute
            .windows(2)
            .all(|pair| pair[0].minute < pair[1].minute),
        "минуты не повторяются и идут по возрастанию"
    );
}

#[tokio::test]
async fn in_memory_counts_messages_per_minute_inside_the_day_window() {
    assert_message_count_contract(&InMemoryTextChatStore::default()).await;
}

#[tokio::test]
async fn in_memory_window_boundary_keeps_the_lower_bound_exclusive() {
    let store = InMemoryTextChatStore::default();
    let now = Utc::now();
    let minute = DateTime::from_timestamp(now.timestamp() - now.timestamp() % 60 - 120, 0)
        .expect("valid minute");
    let store: &dyn TextChatStore = &store;
    for created_at in [
        minute - Duration::hours(24),
        minute - Duration::hours(24) + Duration::seconds(1),
    ] {
        store
            .insert_text_message(TextMessage {
                id: Uuid::new_v4(),
                server_id: Uuid::new_v4(),
                room_id: Uuid::new_v4(),
                author_user_id: Uuid::new_v4(),
                author_nickname: "stats".to_owned(),
                body: "message".to_owned(),
                attachments: Vec::new(),
                created_at,
                deleted_at: None,
                deleted_by_user_id: None,
            })
            .await
            .expect("insert message");
    }

    let per_minute = store
        .count_messages_per_minute(minute - Duration::hours(24), now)
        .await
        .expect("count messages per minute");

    assert_eq!(
        per_minute.len(),
        1,
        "сообщение ровно на границе окна не входит в него"
    );
}

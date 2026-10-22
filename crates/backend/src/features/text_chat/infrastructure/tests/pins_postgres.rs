//! Интеграционная проверка конкурентных закреплений через PostgreSQL.

use std::sync::Arc;

use cheenhub_contracts::rest::ServerRoomKind;
use cheenhub_migrations::{Migrator, MigratorTrait};
use chrono::Utc;
use sea_orm::{ConnectionTrait, Database, DbBackend, DbConn, Statement};
use tokio::task::JoinSet;
use uuid::Uuid;

use super::super::{PIN_LIMIT, PinMessageResult, PostgresTextChatStore, TextChatStore};
use crate::features::auth::domain::RegistrationLegalAcceptance;
use crate::features::auth::infrastructure::{AuthStore, PostgresAuthStore};
use crate::features::servers::domain::ServerRoomWriteAccess;
use crate::features::servers::infrastructure::{PostgresServerStore, ServerStore};
use crate::features::text_chat::domain::TextMessage;

async fn dedicated_database() -> (DbConn, DbConn) {
    let url = std::env::var("CHEENHUB_PINS_TEST_DATABASE_URL")
        .expect("нужна отдельная тестовая БД и CHEENHUB_PINS_TEST_DATABASE_URL");
    let first = Database::connect(&url)
        .await
        .expect("подключение к тестовой PostgreSQL-базе");
    let row = first
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT current_database() AS name".to_owned(),
        ))
        .await
        .expect("проверка имени тестовой базы")
        .expect("PostgreSQL должен вернуть имя базы");
    let name: String = row.try_get("", "name").expect("имя базы должно читаться");
    assert_eq!(
        name, "cheenhub_pins_test",
        "запрещено очищать базу с другим именем"
    );

    Migrator::fresh(&first)
        .await
        .expect("создание чистой схемы тестовой базы");
    let second = Database::connect(&url)
        .await
        .expect("независимое подключение к тестовой базе");
    (first, second)
}

async fn fixture(database: &DbConn) -> (Uuid, Uuid, Uuid) {
    let auth = PostgresAuthStore::new(database.clone());
    let user = auth
        .insert_user(
            "pins_pg_owner".to_owned(),
            "pins-pg-owner@example.test".to_owned(),
            "pins-pg-owner@example.test".to_owned(),
            None,
            RegistrationLegalAcceptance {
                acceptance_source: "registration".to_owned(),
                terms_version: "test".to_owned(),
                privacy_policy_version: "test".to_owned(),
                personal_data_consent_version: "test".to_owned(),
            },
            Utc::now(),
        )
        .await
        .expect("создание тестового пользователя");
    let servers = PostgresServerStore::new(database.clone());
    let server = servers
        .insert_server(&user.id, "Pins PostgreSQL Test".to_owned())
        .await
        .expect("создание тестового сервера");
    let room = servers
        .insert_server_room(
            &server.id,
            "general".to_owned(),
            ServerRoomKind::TextAndVoice,
            ServerRoomWriteAccess::all_members(),
        )
        .await
        .expect("создание тестовой комнаты");
    (user.id, server.id, room.id)
}

async fn insert_message(
    store: &PostgresTextChatStore,
    user_id: Uuid,
    server_id: Uuid,
    room_id: Uuid,
) -> Uuid {
    let id = Uuid::new_v4();
    store
        .insert_text_message(TextMessage {
            id,
            server_id,
            room_id,
            author_user_id: user_id,
            author_nickname: "pins_pg_owner".to_owned(),
            body: "Закрепление для PostgreSQL-теста".to_owned(),
            attachments: Vec::new(),
            created_at: Utc::now(),
            edited_at: None,
            deleted_at: None,
            deleted_by_user_id: None,
        })
        .await
        .expect("сохранение тестового сообщения");
    id
}

#[tokio::test]
#[ignore = "Нужна отдельная PostgreSQL-база cheenhub_pins_test и CHEENHUB_PINS_TEST_DATABASE_URL"]
async fn concurrent_pins_respect_limit_and_deletion_removes_pin() {
    let (first, second) = dedicated_database().await;
    let (user_id, server_id, room_id) = fixture(&first).await;
    let store_a = Arc::new(PostgresTextChatStore::new(first));
    let store_b = Arc::new(PostgresTextChatStore::new(second));
    let mut message_ids = Vec::new();
    for _ in 0..(PIN_LIMIT + 1) {
        message_ids.push(insert_message(&store_a, user_id, server_id, room_id).await);
    }

    // Разные пулы одновременно закрепляют сообщения одной комнаты.
    let mut tasks = JoinSet::new();
    for (index, message_id) in message_ids.into_iter().enumerate() {
        let store = if index % 2 == 0 {
            Arc::clone(&store_a)
        } else {
            Arc::clone(&store_b)
        };
        tasks.spawn(async move {
            store
                .pin_message(&server_id, &room_id, &message_id, &user_id)
                .await
        });
    }

    let mut successful = Vec::new();
    let mut limit_rejections = 0;
    while let Some(outcome) = tasks.join_next().await {
        match outcome
            .expect("задача закрепления должна завершиться")
            .expect("PostgreSQL не должен возвращать ошибку")
        {
            PinMessageResult::Pinned(pin) => successful.push(pin.message.id),
            PinMessageResult::LimitReached => limit_rejections += 1,
            _ => panic!("неожиданный результат конкурентного закрепления"),
        }
    }
    assert_eq!(successful.len(), PIN_LIMIT);
    assert_eq!(limit_rejections, 1);
    let listed = store_a
        .list_pinned_messages(&server_id, &room_id)
        .await
        .expect("получение закреплений");
    assert_eq!(listed.len(), PIN_LIMIT);
    let unique = listed
        .iter()
        .map(|pin| pin.message.id)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(unique.len(), PIN_LIMIT);

    // Повторное закрепление отличается от превышения лимита.
    let duplicate = store_b
        .pin_message(&server_id, &room_id, &successful[0], &user_id)
        .await
        .expect("проверка повторного закрепления");
    assert!(matches!(duplicate, PinMessageResult::AlreadyPinned));

    // Освобождаем место и проверяем параллельное закрепление с удалением.
    assert!(
        store_a
            .unpin_message(&server_id, &room_id, &successful[0])
            .await
            .expect("снятие закрепления")
    );
    let racing_id = insert_message(&store_a, user_id, server_id, room_id).await;
    let (pinned, deleted) = tokio::join!(
        store_a.pin_message(&server_id, &room_id, &racing_id, &user_id),
        store_b.soft_delete_message(&server_id, &room_id, &racing_id, &user_id, true),
    );
    assert!(matches!(
        pinned.expect("закрепление во время удаления"),
        PinMessageResult::Pinned(_) | PinMessageResult::NotFound
    ));
    assert!(deleted.expect("удаление сообщения").is_some());
    let listed = store_a
        .list_pinned_messages(&server_id, &room_id)
        .await
        .expect("проверка закреплений после удаления");
    assert_eq!(listed.len(), PIN_LIMIT - 1);
    assert!(listed.iter().all(|pin| pin.message.id != racing_id));
}

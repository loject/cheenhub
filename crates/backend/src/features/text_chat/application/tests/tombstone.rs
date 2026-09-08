//! Проверки скрытия профиля автора после удаления аккаунта.

use chrono::{Duration, Utc};
use uuid::Uuid;

use super::{registered_user, state};
use crate::features::text_chat::application::history_authors::deleted_authors;
use crate::features::text_chat::domain::TextMessage;

#[tokio::test]
async fn history_author_mask_tracks_deletion_and_restoration() {
    let state = state();
    let auth = registered_user(&state, "author", "author@example.com").await;
    let author_user_id = Uuid::parse_str(&auth.user.id).unwrap();
    let now = Utc::now();
    let messages = vec![TextMessage {
        id: Uuid::new_v4(),
        server_id: Uuid::new_v4(),
        room_id: Uuid::new_v4(),
        author_user_id,
        author_nickname: "Историческое имя".to_owned(),
        body: "Сообщение".to_owned(),
        attachments: Vec::new(),
        created_at: now,
        deleted_at: None,
        deleted_by_user_id: None,
    }];
    assert!(!deleted_authors(&state, &messages).await.unwrap()[&author_user_id]);
    assert!(
        state
            .auth_store
            .begin_account_deletion(
                &author_user_id,
                "restore-hash".to_owned(),
                now,
                now + Duration::days(30),
            )
            .await
            .unwrap()
    );
    assert!(deleted_authors(&state, &messages).await.unwrap()[&author_user_id]);
    assert!(
        state
            .auth_store
            .restore_account("restore-hash", now + Duration::days(1))
            .await
            .unwrap()
    );
    assert!(!deleted_authors(&state, &messages).await.unwrap()[&author_user_id]);
}

use cheenhub_contracts::realtime::{DeleteMessage, LoadRoomHistory};
use cheenhub_contracts::rest::ServerRoomKind;
use chrono::Utc;
use uuid::Uuid;

use super::super::{TextChatApplicationError, delete_message, load_room_history};
use super::{create_server_room, registered_user, state};
use crate::features::text_chat::domain::TextMessage;

#[tokio::test]
async fn cannot_delete_message_from_foreign_server_via_owned_room() {
    let state = state();
    let attacker = registered_user(&state, "attacker", "attacker@example.com").await;
    let victim = registered_user(&state, "victim_del", "victim-del@example.com").await;
    let attacker_id = Uuid::parse_str(&attacker.user.id).expect("user id should be uuid");
    let victim_id = Uuid::parse_str(&victim.user.id).expect("user id should be uuid");

    // Атакующий владеет своим сервером и текстовой комнатой.
    let (attacker_server, attacker_room) = create_server_room(
        &state,
        &attacker_id,
        "Attacker",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;

    // Жертва владеет отдельным сервером с сообщением, автор которого - жертва.
    let (victim_server_string, victim_room_string) = create_server_room(
        &state,
        &victim_id,
        "Victim",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let victim_server = Uuid::parse_str(&victim_server_string).expect("server id should be uuid");
    let victim_room = Uuid::parse_str(&victim_room_string).expect("room id should be uuid");
    let foreign_message_id = Uuid::new_v4();
    state
        .text_chat_store
        .insert_text_message(TextMessage {
            id: foreign_message_id,
            server_id: victim_server,
            room_id: victim_room,
            author_user_id: victim_id,
            author_nickname: victim.user.nickname.clone(),
            body: "secret".to_owned(),
            attachments: Vec::new(),
            created_at: Utc::now(),
            deleted_at: None,
            deleted_by_user_id: None,
        })
        .await
        .expect("message should insert");

    // Права атакующего не должны применяться к сообщению из другой комнаты.
    let error = delete_message(
        &state,
        &attacker_id,
        DeleteMessage {
            server_id: attacker_server,
            room_id: attacker_room,
            message_id: foreign_message_id.to_string(),
        },
    )
    .await
    .expect_err("cross-server delete must be rejected");
    assert!(matches!(error, TextChatApplicationError::NotFound(_)));

    // Сообщение жертвы должно остаться нетронутым.
    let history = load_room_history(
        &state,
        &victim_id,
        LoadRoomHistory {
            server_id: victim_server_string,
            room_id: victim_room_string,
            before_message_id: None,
        },
    )
    .await
    .expect("victim history should load");
    assert_eq!(history.messages.len(), 1);
    assert_eq!(history.messages[0].id, foreign_message_id.to_string());
}

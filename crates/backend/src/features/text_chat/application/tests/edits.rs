//! Сценарии редактирования текста сообщений комнат.

use cheenhub_contracts::realtime::{DeleteMessage, EditMessage, LoadRoomHistory};
use cheenhub_contracts::rest::ServerRoomKind;
use chrono::Utc;
use uuid::Uuid;

use super::super::{TextChatApplicationError, delete_message, edit_message, load_room_history};
use super::{create_server_room, registered_user, state};
use crate::features::text_chat::domain::{ChatAttachment, TextMessage};
use crate::state::AppState;

async fn insert_message(
    state: &AppState,
    server_id: &str,
    room_id: &str,
    author: Uuid,
    attachments: Vec<ChatAttachment>,
) -> Uuid {
    let id = Uuid::new_v4();
    state
        .text_chat_store
        .insert_text_message(TextMessage {
            id,
            server_id: Uuid::parse_str(server_id).unwrap(),
            room_id: Uuid::parse_str(room_id).unwrap(),
            author_user_id: author,
            author_nickname: "Автор".to_owned(),
            body: "Исходный текст".to_owned(),
            attachments,
            created_at: Utc::now(),
            edited_at: None,
            deleted_at: None,
            deleted_by_user_id: None,
        })
        .await
        .expect("message must be inserted");
    id
}

fn request(server_id: &str, room_id: &str, message_id: Uuid, body: &str) -> EditMessage {
    EditMessage {
        server_id: server_id.to_owned(),
        room_id: room_id.to_owned(),
        message_id: message_id.to_string(),
        body: body.to_owned(),
    }
}

#[tokio::test]
async fn author_edit_persists_in_history_without_changing_message_identity() {
    let state = state();
    let author = registered_user(&state, "edit_author", "edit-author@example.com").await;
    let author_id = Uuid::parse_str(&author.user.id).unwrap();
    let (server_id, room_id) = create_server_room(
        &state,
        &author_id,
        "Edits",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let message_id = insert_message(&state, &server_id, &room_id, author_id, vec![]).await;

    let result = edit_message(
        &state,
        &author.user,
        &author_id,
        request(&server_id, &room_id, message_id, "  Новый текст  "),
    )
    .await
    .expect("author can edit message");

    assert_eq!(result.message.id, message_id.to_string());
    assert_eq!(result.message.body, "Новый текст");
    assert!(result.message.edited_at.is_some());
    let history = load_room_history(
        &state,
        &author_id,
        LoadRoomHistory {
            server_id,
            room_id,
            before_message_id: None,
        },
    )
    .await
    .expect("history must load");
    assert_eq!(history.messages.len(), 1);
    assert_eq!(history.messages[0].body, "Новый текст");
    assert_eq!(history.messages[0].edited_at, result.message.edited_at);
    assert_eq!(history.messages[0].created_at, result.message.created_at);
}

#[tokio::test]
async fn other_server_member_cannot_edit_authors_message() {
    let state = state();
    let author = registered_user(&state, "owner_edit", "owner-edit@example.com").await;
    let other = registered_user(&state, "other_edit", "other-edit@example.com").await;
    let author_id = Uuid::parse_str(&author.user.id).unwrap();
    let other_id = Uuid::parse_str(&other.user.id).unwrap();
    let (server_id, room_id) = create_server_room(
        &state,
        &author_id,
        "Edits",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    state
        .server_store
        .insert_server_member(&Uuid::parse_str(&server_id).unwrap(), &other_id)
        .await
        .expect("other user joins server");
    let message_id = insert_message(&state, &server_id, &room_id, author_id, vec![]).await;

    let error = edit_message(
        &state,
        &other.user,
        &other_id,
        request(&server_id, &room_id, message_id, "Подмена"),
    )
    .await
    .expect_err("other member cannot edit");

    assert!(matches!(error, TextChatApplicationError::NotFound(_)));
}

#[tokio::test]
async fn author_cannot_edit_message_using_another_rooms_identifiers() {
    let state = state();
    let author = registered_user(&state, "cross_edit", "cross-edit@example.com").await;
    let author_id = Uuid::parse_str(&author.user.id).unwrap();
    let (first_server, first_room) = create_server_room(
        &state,
        &author_id,
        "First",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let (other_server, other_room) = create_server_room(
        &state,
        &author_id,
        "Second",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let message_id = insert_message(&state, &other_server, &other_room, author_id, vec![]).await;

    let error = edit_message(
        &state,
        &author.user,
        &author_id,
        request(&first_server, &first_room, message_id, "Чужая комната"),
    )
    .await
    .expect_err("cross-room edit must be rejected");

    assert!(matches!(error, TextChatApplicationError::NotFound(_)));
}

#[tokio::test]
async fn deleted_message_cannot_be_edited() {
    let state = state();
    let author = registered_user(&state, "delete_edit", "delete-edit@example.com").await;
    let author_id = Uuid::parse_str(&author.user.id).unwrap();
    let (server_id, room_id) = create_server_room(
        &state,
        &author_id,
        "Edits",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let message_id = insert_message(&state, &server_id, &room_id, author_id, vec![]).await;
    delete_message(
        &state,
        &author_id,
        DeleteMessage {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            message_id: message_id.to_string(),
        },
    )
    .await
    .expect("author can delete");

    let error = edit_message(
        &state,
        &author.user,
        &author_id,
        request(&server_id, &room_id, message_id, "После удаления"),
    )
    .await
    .expect_err("deleted message cannot be edited");

    assert!(matches!(error, TextChatApplicationError::NotFound(_)));
}

#[tokio::test]
async fn empty_text_without_attachment_is_rejected() {
    let state = state();
    let author = registered_user(&state, "empty_edit", "empty-edit@example.com").await;
    let author_id = Uuid::parse_str(&author.user.id).unwrap();
    let (server_id, room_id) = create_server_room(
        &state,
        &author_id,
        "Edits",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let message_id = insert_message(&state, &server_id, &room_id, author_id, vec![]).await;

    let error = edit_message(
        &state,
        &author.user,
        &author_id,
        request(&server_id, &room_id, message_id, "   "),
    )
    .await
    .expect_err("empty text-only message is invalid");

    assert!(matches!(error, TextChatApplicationError::BadRequest(_)));
}

#[tokio::test]
async fn image_caption_can_be_cleared_without_changing_attachment() {
    let state = state();
    let author = registered_user(&state, "image_edit", "image-edit@example.com").await;
    let author_id = Uuid::parse_str(&author.user.id).unwrap();
    let (server_id, room_id) = create_server_room(
        &state,
        &author_id,
        "Edits",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let attachment_id = Uuid::new_v4();
    let attachment = ChatAttachment {
        id: attachment_id,
        server_id: Uuid::parse_str(&server_id).unwrap(),
        room_id: Uuid::parse_str(&room_id).unwrap(),
        uploader_user_id: author_id,
        message_id: None,
        bucket: "test".to_owned(),
        object_key: "image".to_owned(),
        content_type: "image/png".to_owned(),
        byte_size: 4,
        width: 1,
        height: 1,
        sha256: "test".to_owned(),
        original_filename: None,
        created_at: Utc::now(),
    };
    let message_id =
        insert_message(&state, &server_id, &room_id, author_id, vec![attachment]).await;

    let result = edit_message(
        &state,
        &author.user,
        &author_id,
        request(&server_id, &room_id, message_id, "  "),
    )
    .await
    .expect("image caption may be empty");

    assert_eq!(result.message.body, "");
    assert_eq!(result.message.attachments.len(), 1);
    assert_eq!(result.message.attachments[0].id, attachment_id.to_string());
    assert!(result.message.edited_at.is_some());
}

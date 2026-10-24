//! Сценарии команд, корреляции ответов и отказов доступа ЛС.

use super::*;
use cheenhub_contracts::rest::{
    EditDmMessageResponse, ListDmConversationsResponse, ListDmMessagesResponse,
    MarkDmConversationReadResponse, OpenDmConversationResponse, SendDmMessageResponse,
    UploadDmImageResponse,
};
use serde_json::json;

#[tokio::test]
async fn conversations_list_and_open_return_existing_dialog() {
    let pair = pair().await;
    let listed = command(
        &pair.state,
        &pair.alice,
        "list_direct_message_conversations",
        serde_json::Value::Null,
        Some(Uuid::new_v4()),
    )
    .await;
    let listed: ListDmConversationsResponse = response(listed, "direct_message_conversations");
    assert_eq!(listed.conversations.len(), 1);
    assert_eq!(listed.conversations[0].id, pair.conversation_id);

    let opened = command(
        &pair.state,
        &pair.alice,
        "open_direct_message_conversation",
        json!({"friend_user_id": pair.bob.id}),
        Some(Uuid::new_v4()),
    )
    .await;
    let opened: OpenDmConversationResponse = response(opened, "direct_message_conversation_opened");
    assert_eq!(opened.conversation.id, pair.conversation_id);
}

#[tokio::test]
async fn send_uses_authenticated_author_and_history_cursor_returns_older_messages() {
    let pair = pair().await;
    let first = send(&pair, &pair.bob, "Первое").await;

    let sent = command(
        &pair.state,
        &pair.alice,
        "send_direct_message",
        json!({
            "conversation_id": pair.conversation_id, "body": "Второе", "image_id": null,
            "sender_user_id": pair.bob.id
        }),
        Some(Uuid::new_v4()),
    )
    .await;

    let sent: SendDmMessageResponse = response(sent, "send_direct_message_accepted");
    assert_eq!(sent.message.sender_user_id, pair.alice.id);
    assert_eq!(sent.message.body, "Второе");
    assert!(sent.message.seq > first.seq);
    let history = command(
        &pair.state,
        &pair.alice,
        "load_direct_message_history",
        json!({
            "conversation_id": pair.conversation_id, "before_message_id": sent.message.id,
        }),
        Some(Uuid::new_v4()),
    )
    .await;
    let history: ListDmMessagesResponse = response(history, "direct_message_history");
    assert_eq!(history.messages.len(), 1);
    assert_eq!(history.messages[0].id, first.id);
    assert!(!history.has_more);
}

#[tokio::test]
async fn edit_accepts_author_and_rejects_recipient() {
    let pair = pair().await;
    let sent = send(&pair, &pair.alice, "Исходный").await;
    let payload =
        json!({"conversation_id": pair.conversation_id, "message_id": sent.id, "body": "Новый"});

    let denied = command(
        &pair.state,
        &pair.bob,
        "edit_direct_message",
        payload.clone(),
        Some(Uuid::new_v4()),
    )
    .await;
    rejection(denied, RejectionCode::BadRequest);
    let edited = command(
        &pair.state,
        &pair.alice,
        "edit_direct_message",
        payload,
        Some(Uuid::new_v4()),
    )
    .await;

    let edited: EditDmMessageResponse = response(edited, "edit_direct_message_accepted");
    assert_eq!(edited.message.body, "Новый");
    assert_eq!(edited.message.seq, sent.seq);
    assert!(edited.message.edited_at.is_some());
}

#[tokio::test]
async fn read_checkpoint_clears_incoming_unread_count() {
    let pair = pair().await;
    let sent = send(&pair, &pair.bob, "Прочитай").await;

    let read = command(
        &pair.state,
        &pair.alice,
        "mark_direct_message_read",
        json!({
            "conversation_id": pair.conversation_id, "last_read_message_id": sent.id
        }),
        Some(Uuid::new_v4()),
    )
    .await;

    let read: MarkDmConversationReadResponse = response(read, "mark_direct_message_read_accepted");
    assert_eq!(read.last_read_seq, sent.seq);
    assert_eq!(read.conversation_unread_count, 0);
    assert_eq!(read.total_unread_count, 0);
}

#[tokio::test]
async fn outsider_cannot_read_or_write_conversation() {
    let pair = pair().await;
    let outsider = register(&pair.state, "outsider_rt", "outsider-rt@example.com").await;
    for (kind, payload) in [
        (
            "load_direct_message_history",
            json!({"conversation_id": pair.conversation_id}),
        ),
        (
            "send_direct_message",
            json!({"conversation_id": pair.conversation_id, "body": "Чужое"}),
        ),
    ] {
        let denied = command(
            &pair.state,
            &outsider.user,
            kind,
            payload,
            Some(Uuid::new_v4()),
        )
        .await;
        rejection(denied, RejectionCode::BadRequest);
    }
}

#[tokio::test]
async fn malformed_payload_and_missing_request_id_return_bad_request() {
    let pair = pair().await;
    for (payload, request_id) in [
        (json!({}), Some(Uuid::new_v4())),
        (
            json!({"conversation_id": pair.conversation_id, "body": "Не сохранять"}),
            None,
        ),
        (
            json!({"conversation_id": "invalid", "body": "Не сохранять"}),
            Some(Uuid::new_v4()),
        ),
    ] {
        let denied = command(
            &pair.state,
            &pair.alice,
            "send_direct_message",
            payload,
            request_id,
        )
        .await;
        rejection(denied, RejectionCode::BadRequest);
    }
    let history =
        application::list_dm_messages(&pair.state, &id(&pair.alice), pair.conversation_id, None)
            .await
            .unwrap();
    assert!(history.messages.is_empty());
}

#[tokio::test]
async fn image_upload_attach_and_load_preserve_bytes_and_access_scope() {
    use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
    let pair = pair().await;
    let image = image::ImageBuffer::from_pixel(2, 2, image::Rgba([40_u8, 120, 220, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    let bytes = bytes.into_inner();

    let uploaded = command(
        &pair.state,
        &pair.alice,
        "upload_direct_message_image",
        json!({
            "conversation_id": pair.conversation_id, "data_base64": BASE64.encode(&bytes),
        }),
        Some(Uuid::new_v4()),
    )
    .await;
    let uploaded: UploadDmImageResponse =
        response(uploaded, "upload_direct_message_image_accepted");
    let loaded = command(
        &pair.state,
        &pair.bob,
        "load_direct_message_image",
        json!({
            "conversation_id": pair.conversation_id, "image_id": uploaded.image.id
        }),
        Some(Uuid::new_v4()),
    )
    .await;
    rejection(loaded, RejectionCode::BadRequest);
    let sent = command(
        &pair.state,
        &pair.alice,
        "send_direct_message",
        json!({
            "conversation_id": pair.conversation_id, "body": "", "image_id": uploaded.image.id,
        }),
        Some(Uuid::new_v4()),
    )
    .await;
    let sent: SendDmMessageResponse = response(sent, "send_direct_message_accepted");
    assert_eq!(sent.message.image.unwrap().id, uploaded.image.id);

    let loaded = command(
        &pair.state,
        &pair.bob,
        "load_direct_message_image",
        json!({
            "conversation_id": pair.conversation_id, "image_id": uploaded.image.id
        }),
        Some(Uuid::new_v4()),
    )
    .await;
    assert_eq!(
        serde_json::to_value(loaded.kind).unwrap(),
        "direct_message_image_loaded"
    );
    assert_eq!(loaded.payload["id"], uploaded.image.id);
    assert_eq!(loaded.payload["content_type"], "image/png");
    assert_eq!(
        BASE64
            .decode(loaded.payload["data_base64"].as_str().unwrap())
            .unwrap(),
        bytes
    );
}

#[tokio::test]
async fn invalid_base64_returns_bad_request() {
    let pair = pair().await;

    let denied = command(
        &pair.state,
        &pair.alice,
        "upload_direct_message_image",
        json!({
            "conversation_id": pair.conversation_id, "data_base64": "not base64!",
        }),
        Some(Uuid::new_v4()),
    )
    .await;

    rejection(denied, RejectionCode::BadRequest);
}

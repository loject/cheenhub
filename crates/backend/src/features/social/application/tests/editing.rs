//! Проверки изменения текста личных сообщений без изменения вложений и read-state.

use super::*;
use crate::features::social::{SocialError, application::edit_dm_message};
use cheenhub_contracts::rest::EditDmMessageRequest;

#[tokio::test]
async fn author_edit_updates_history_without_changing_delivery_state() {
    let setup = setup_pair().await;
    let sent = send_dm_message(
        &setup.state,
        &user_id(&setup.bob_user_id),
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "Исходный текст".to_owned(),
            image_id: None,
        },
    )
    .await
    .expect("message should send")
    .message;
    let unread_before = conversation_unread(&setup).await;

    let edited = edit_dm_message(
        &setup.state,
        &user_id(&setup.bob_user_id),
        setup.conversation_id.clone(),
        sent.id.clone(),
        EditDmMessageRequest {
            body: "  Новый текст  ".to_owned(),
        },
    )
    .await
    .expect("author should edit message")
    .message;

    assert_eq!(edited.id, sent.id);
    assert_eq!(edited.seq, sent.seq);
    assert_eq!(edited.created_at, sent.created_at);
    assert_eq!(edited.body, "Новый текст");
    assert!(edited.edited_at.is_some());
    assert_eq!(edited.image, sent.image);
    assert_eq!(conversation_unread(&setup).await, unread_before);

    let recipient_history = list_dm_messages(
        &setup.state,
        &user_id(&setup.alice_user_id),
        setup.conversation_id.clone(),
        None,
    )
    .await
    .expect("recipient should load edited message");
    assert_eq!(recipient_history.messages[0].body, "Новый текст");
    assert_eq!(recipient_history.messages[0].edited_at, edited.edited_at);

    let repeated = edit_dm_message(
        &setup.state,
        &user_id(&setup.bob_user_id),
        setup.conversation_id,
        sent.id,
        EditDmMessageRequest {
            body: "Новый текст".to_owned(),
        },
    )
    .await
    .expect("repeated edit should succeed");
    assert_eq!(repeated.message.edited_at, edited.edited_at);
}

#[tokio::test]
async fn recipient_cannot_edit_another_users_message() {
    let setup = setup_pair().await;
    let sent = send_dm_message(
        &setup.state,
        &user_id(&setup.bob_user_id),
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "Только мой текст".to_owned(),
            image_id: None,
        },
    )
    .await
    .expect("message should send")
    .message;

    let denied = edit_dm_message(
        &setup.state,
        &user_id(&setup.alice_user_id),
        setup.conversation_id.clone(),
        sent.id,
        EditDmMessageRequest {
            body: "Попытка изменения".to_owned(),
        },
    )
    .await
    .expect_err("recipient must not edit another author's message");
    assert!(matches!(denied, SocialError::NotFound(_)));

    let history = list_dm_messages(
        &setup.state,
        &user_id(&setup.alice_user_id),
        setup.conversation_id,
        None,
    )
    .await
    .expect("history should remain readable");
    assert_eq!(history.messages[0].body, "Только мой текст");
    assert!(history.messages[0].edited_at.is_none());
}

#[tokio::test]
async fn editing_text_only_message_rejects_blank_body() {
    let setup = setup_pair().await;
    let sent = send_dm_message(
        &setup.state,
        &user_id(&setup.alice_user_id),
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "Не пустое сообщение".to_owned(),
            image_id: None,
        },
    )
    .await
    .expect("message should send")
    .message;

    let error = edit_dm_message(
        &setup.state,
        &user_id(&setup.alice_user_id),
        setup.conversation_id,
        sent.id,
        EditDmMessageRequest {
            body: "   ".to_owned(),
        },
    )
    .await
    .expect_err("message without an image cannot become empty");
    assert!(matches!(error, SocialError::BadRequest(_)));
}

#[tokio::test]
async fn editing_image_caption_keeps_attachment_and_allows_empty_caption() {
    let setup = setup_pair().await;
    let image = upload_dm_image(
        &setup.state,
        &user_id(&setup.alice_user_id),
        setup.conversation_id.clone(),
        Bytes::from(png_bytes()),
    )
    .await
    .expect("image should upload")
    .image;
    let sent = send_dm_message(
        &setup.state,
        &user_id(&setup.alice_user_id),
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: String::new(),
            image_id: Some(image.id.clone()),
        },
    )
    .await
    .expect("image message should send")
    .message;

    let captioned = edit_dm_message(
        &setup.state,
        &user_id(&setup.alice_user_id),
        setup.conversation_id.clone(),
        sent.id.clone(),
        EditDmMessageRequest {
            body: "Подпись".to_owned(),
        },
    )
    .await
    .expect("author should add caption")
    .message;
    assert_eq!(captioned.body, "Подпись");
    assert_eq!(captioned.image, sent.image);

    let cleared = edit_dm_message(
        &setup.state,
        &user_id(&setup.alice_user_id),
        setup.conversation_id,
        sent.id,
        EditDmMessageRequest {
            body: String::new(),
        },
    )
    .await
    .expect("author should remove caption")
    .message;
    assert!(cleared.body.is_empty());
    assert_eq!(cleared.image, sent.image);
    assert!(cleared.edited_at.is_some());
}

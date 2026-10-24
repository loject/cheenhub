//! Проверки выполнения ЛС от имени проверенного пользователя без токена.

use super::*;
use crate::features::social::SocialError;
use uuid::Uuid;

#[tokio::test]
async fn identified_sender_sends_message_without_access_token() {
    let setup = setup_pair().await;
    let user_id = Uuid::parse_str(&setup.alice_user_id).expect("valid user id");

    let sent = send_dm_message(
        &setup.state,
        &user_id,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "Без передачи токена".to_owned(),
            image_id: None,
        },
    )
    .await
    .expect("identified user should send");

    assert_eq!(sent.message.sender_user_id, setup.alice_user_id);
    assert_eq!(sent.message.sender_nickname, "alice_dm");
    assert_eq!(sent.message.body, "Без передачи токена");
    let history = list_dm_messages(&setup.state, &user_id, setup.conversation_id, None)
        .await
        .expect("identified user should read history");
    assert_eq!(history.messages, vec![sent.message]);
}

#[tokio::test]
async fn identified_outsider_cannot_read_or_write_foreign_conversation() {
    let setup = setup_pair().await;
    let outsider = registered_user(&setup.state, "outsider_dm", "outsider-dm@example.com").await;
    let user_id = Uuid::parse_str(&outsider.user.id).expect("valid user id");

    let history =
        list_dm_messages(&setup.state, &user_id, setup.conversation_id.clone(), None).await;
    let send = send_dm_message(
        &setup.state,
        &user_id,
        setup.conversation_id,
        SendDmMessageRequest {
            body: "Чужой диалог".to_owned(),
            image_id: None,
        },
    )
    .await;

    assert!(matches!(history, Err(SocialError::NotFound(_))));
    assert!(matches!(send, Err(SocialError::NotFound(_))));
}

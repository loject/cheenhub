//! Проверки запрета взаимодействий с tombstone при сохранении истории.

use super::*;
use crate::features::social::SocialError;
use uuid::Uuid;

#[tokio::test]
async fn deleted_friend_rejects_new_actions_but_preserves_history() {
    let setup = setup_pair().await;
    send_dm_message(
        &setup.state,
        &setup.alice_access_token,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            image_id: None,
            body: "Сохранённая история".to_owned(),
        },
    )
    .await
    .expect("message should send before deletion");
    let alice_id = Uuid::parse_str(&setup.alice_user_id).expect("valid user UUID");
    let now = chrono::Utc::now();
    assert!(
        setup
            .state
            .auth_store
            .begin_account_deletion(
                &alice_id,
                "restore-token-hash".to_owned(),
                now,
                now + chrono::Duration::days(30),
            )
            .await
            .expect("deletion should start")
    );

    let denied = send_dm_message(
        &setup.state,
        &setup.bob_access_token,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            image_id: None,
            body: "Новое сообщение".to_owned(),
        },
    )
    .await
    .expect_err("must reject message to tombstone");
    assert_deleted(denied);
    assert_deleted(
        open_dm_conversation(
            &setup.state,
            &setup.bob_access_token,
            OpenDmConversationRequest {
                friend_user_id: setup.alice_user_id.clone(),
            },
        )
        .await
        .expect_err("must reject opening conversation with tombstone"),
    );
    assert_deleted(
        send_friend_request(
            &setup.state,
            &setup.bob_access_token,
            SendFriendRequestRequest {
                recipient_user_id: setup.alice_user_id.clone(),
            },
        )
        .await
        .expect_err("must reject friend request to tombstone"),
    );
    let bob = auth_application::require_current_user(&setup.state, &setup.bob_access_token)
        .await
        .expect("remaining user is active")
        .0;
    assert_deleted(
        crate::features::social::application::direct_message_voice_access(
            &setup.state,
            &bob.id,
            &Uuid::parse_str(&setup.conversation_id).expect("valid conversation UUID"),
        )
        .await
        .expect_err("must reject direct call to tombstone"),
    );
    let history = list_dm_messages(
        &setup.state,
        &setup.bob_access_token,
        setup.conversation_id.clone(),
        None,
    )
    .await
    .expect("history remains readable");
    assert_eq!(history.messages.len(), 1);
    assert_eq!(history.messages[0].body, "Сохранённая история");
    assert_ne!(history.messages[0].sender_nickname, "alice_dm");
    assert!(history.messages[0].sender_avatar_url.is_none());
    assert_eq!(
        list_dm_conversations(&setup.state, &setup.bob_access_token)
            .await
            .expect("existing conversations remain readable")
            .conversations
            .len(),
        1
    );

    assert!(
        setup
            .state
            .auth_store
            .restore_account("restore-token-hash", now)
            .await
            .expect("account should restore")
    );
    send_dm_message(
        &setup.state,
        &setup.bob_access_token,
        setup.conversation_id,
        SendDmMessageRequest {
            image_id: None,
            body: "С возвращением".to_owned(),
        },
    )
    .await
    .expect("restoration enables new messages");
}

fn assert_deleted(error: SocialError) {
    assert!(matches!(error, SocialError::BadRequest(message) if message == "Этот аккаунт удалён."));
}

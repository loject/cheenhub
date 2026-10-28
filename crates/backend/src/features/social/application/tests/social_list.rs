//! Проверки объединённого списка друзей и собеседников.

use cheenhub_contracts::realtime::ListSocialEntries;
use cheenhub_contracts::rest::UserRelationStatus;

use super::*;

#[tokio::test]
async fn social_list_deduplicates_friends_and_retains_nonfriend_conversations() {
    let setup = setup_pair().await;
    send_dm_message(
        &setup.state,
        &user_id(&setup.bob_user_id),
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "Привет от Боба".to_owned(),
            image_id: None,
        },
    )
    .await
    .unwrap();

    let charlie =
        registered_user(&setup.state, "charlie_social", "charlie-social@example.com").await;
    befriend(&setup, &charlie).await;
    let dave = registered_user(&setup.state, "dave_social", "dave-social@example.com").await;
    befriend(&setup, &dave).await;
    let dave_conversation = open_dm_conversation(
        &setup.state,
        &user_id(&setup.alice_user_id),
        OpenDmConversationRequest {
            friend_user_id: dave.user.id.clone(),
        },
    )
    .await
    .unwrap()
    .conversation;
    super::super::delete_friend(
        &setup.state,
        &setup.alice_access_token,
        dave.user.id.clone(),
    )
    .await
    .unwrap();
    registered_user(
        &setup.state,
        "unrelated_social",
        "unrelated-social@example.com",
    )
    .await;

    let page = list_social_entries(
        &setup.state,
        &user_id(&setup.alice_user_id),
        ListSocialEntries::default(),
    )
    .await
    .expect("unified page should load");

    assert_eq!(page.entries.len(), 3);
    assert!(!page.has_more);
    assert_eq!(page.entries[0].user_id, setup.bob_user_id);
    assert_eq!(page.entries[0].relation, Some(UserRelationStatus::Friends));
    assert_eq!(
        page.entries[0].last_message.as_ref().unwrap().body,
        "Привет от Боба"
    );
    assert!(page.entries[0].conversation.is_some());
    assert_eq!(page.entries[1].user_id, charlie.user.id);
    assert_eq!(page.entries[1].relation, Some(UserRelationStatus::Friends));
    assert!(page.entries[1].conversation.is_none());
    assert!(page.entries[1].last_message.is_none());
    assert_eq!(page.entries[2].user_id, dave.user.id);
    assert_eq!(page.entries[2].relation, None);
    assert_eq!(
        page.entries[2].conversation.as_ref().unwrap().id,
        dave_conversation.id
    );
    assert_eq!(page.entries[2].presence, None);
}

#[tokio::test]
async fn social_list_search_filters_before_cursor_pagination_and_binds_query() {
    let setup = setup_pair().await;
    let alice = registered_user(&setup.state, "ALICE_social", "alice-social@example.com").await;
    let alina = registered_user(&setup.state, "alina_social", "alina-social@example.com").await;
    befriend(&setup, &alice).await;
    befriend(&setup, &alina).await;
    registered_user(
        &setup.state,
        "alike_unrelated",
        "alike-unrelated@example.com",
    )
    .await;

    let first = list_social_entries(
        &setup.state,
        &user_id(&setup.alice_user_id),
        ListSocialEntries {
            query: Some(" ALI ".to_owned()),
            cursor: None,
            limit: Some(1),
        },
    )
    .await
    .expect("first matching page should load");
    assert_eq!(first.entries.len(), 1);
    assert_eq!(first.entries[0].user_id, alice.user.id);
    assert!(first.has_more);

    let second = list_social_entries(
        &setup.state,
        &user_id(&setup.alice_user_id),
        ListSocialEntries {
            query: Some("ali".to_owned()),
            cursor: first.next_cursor.clone(),
            limit: Some(1),
        },
    )
    .await
    .expect("normalized search should accept same cursor");
    assert_eq!(second.entries.len(), 1);
    assert_eq!(second.entries[0].user_id, alina.user.id);
    assert!(!second.has_more);
    assert!(second.next_cursor.is_none());

    let rejected = list_social_entries(
        &setup.state,
        &user_id(&setup.alice_user_id),
        ListSocialEntries {
            query: Some("bob".to_owned()),
            cursor: first.next_cursor,
            limit: Some(1),
        },
    )
    .await
    .expect_err("cursor must not be reused for another search");
    assert!(matches!(
        rejected,
        crate::features::social::SocialError::BadRequest(_)
    ));
}

#[tokio::test]
async fn social_list_cursor_handles_equal_message_times_and_friends_without_messages() {
    use crate::features::social::domain::DmMessage;

    let setup = setup_pair().await;
    let charlie = registered_user(&setup.state, "charlie_equal", "charlie-equal@example.com").await;
    befriend(&setup, &charlie).await;
    let charlie_conversation = open_dm_conversation(
        &setup.state,
        &user_id(&setup.alice_user_id),
        OpenDmConversationRequest {
            friend_user_id: charlie.user.id.clone(),
        },
    )
    .await
    .unwrap()
    .conversation;
    let dave = registered_user(&setup.state, "dave_equal", "dave-equal@example.com").await;
    befriend(&setup, &dave).await;
    let same_time = chrono::Utc::now();
    for (conversation_id, sender_id) in [
        (&setup.conversation_id, &setup.bob_user_id),
        (&charlie_conversation.id, &charlie.user.id),
    ] {
        setup
            .state
            .social_store
            .insert_dm_message(DmMessage {
                id: uuid::Uuid::new_v4(),
                conversation_id: user_id(conversation_id),
                seq: 0,
                sender_user_id: user_id(sender_id),
                body: "Одинаковое время".to_owned(),
                image_id: None,
                created_at: same_time,
                updated_at: same_time,
                deleted_at: None,
            })
            .await
            .unwrap();
    }

    let mut cursor = None;
    let mut actual_ids = Vec::new();
    for expected_has_more in [true, true, false] {
        let page = list_social_entries(
            &setup.state,
            &user_id(&setup.alice_user_id),
            ListSocialEntries {
                query: None,
                cursor,
                limit: Some(1),
            },
        )
        .await
        .unwrap();
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.has_more, expected_has_more);
        if actual_ids.len() < 2 {
            assert_eq!(
                page.entries[0].conversation.as_ref().unwrap().unread_count,
                1
            );
        }
        actual_ids.push(page.entries[0].user_id.clone());
        cursor = page.next_cursor;
    }
    assert_eq!(
        actual_ids,
        vec![setup.bob_user_id, charlie.user.id, dave.user.id]
    );
    assert!(cursor.is_none());
}

#[tokio::test]
async fn social_list_rejects_invalid_limits_and_malformed_cursor() {
    let setup = setup_pair().await;
    for request in [
        ListSocialEntries {
            query: None,
            cursor: None,
            limit: Some(0),
        },
        ListSocialEntries {
            query: None,
            cursor: None,
            limit: Some(101),
        },
        ListSocialEntries {
            query: None,
            cursor: Some("invalid-cursor".to_owned()),
            limit: None,
        },
    ] {
        let result =
            list_social_entries(&setup.state, &user_id(&setup.alice_user_id), request).await;
        assert!(matches!(
            result,
            Err(crate::features::social::SocialError::BadRequest(_))
        ));
    }
}

async fn befriend(setup: &PairSetup, recipient: &cheenhub_contracts::rest::AuthResponse) {
    let request = send_friend_request(
        &setup.state,
        &setup.alice_access_token,
        SendFriendRequestRequest {
            recipient_user_id: recipient.user.id.clone(),
        },
    )
    .await
    .unwrap();
    accept_friend_request(&setup.state, &recipient.access_token, request.request.id)
        .await
        .unwrap();
}

use cheenhub_contracts::rest::ListFriendsQuery;

use super::*;

#[tokio::test]
async fn friends_are_cursor_paginated_by_latest_message_then_identifier() {
    let setup = setup_pair().await;
    send_dm_message(
        &setup.state,
        &setup.bob_access_token,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            image_id: None,
            body: "Более раннее".to_owned(),
        },
    )
    .await
    .expect("older message should send");

    let charlie = registered_user(&setup.state, "charlie_page", "charlie-page@example.com").await;
    let charlie_conversation = befriend_and_open(
        &setup.state,
        &setup.alice_access_token,
        &charlie.access_token,
        &charlie.user.id,
    )
    .await;
    send_dm_message(
        &setup.state,
        &charlie.access_token,
        charlie_conversation,
        SendDmMessageRequest {
            image_id: None,
            body: "Самое новое".to_owned(),
        },
    )
    .await
    .expect("newer message should send");

    let dave = registered_user(&setup.state, "dave_page", "dave-page@example.com").await;
    let request = send_friend_request(
        &setup.state,
        &setup.alice_access_token,
        SendFriendRequestRequest {
            recipient_user_id: dave.user.id.clone(),
        },
    )
    .await
    .expect("friend request should send");
    accept_friend_request(&setup.state, &dave.access_token, request.request.id)
        .await
        .expect("friend request should accept");

    let first = page(&setup, None).await;
    assert_eq!(first.friends[0].user_id, charlie.user.id);
    assert_eq!(last_body(&first), Some("Самое новое"));
    assert!(first.has_more);

    let second = page(&setup, first.next_cursor).await;
    assert_eq!(last_body(&second), Some("Более раннее"));
    assert!(second.has_more);

    let third = page(&setup, second.next_cursor).await;
    assert_eq!(third.friends[0].user_id, dave.user.id);
    assert!(third.friends[0].last_message.is_none());
    assert!(!third.has_more);
    assert!(third.next_cursor.is_none());
}

#[tokio::test]
async fn friends_reject_invalid_pagination_parameters() {
    let setup = setup_pair().await;
    for query in [
        ListFriendsQuery {
            limit: Some(0),
            cursor: None,
        },
        ListFriendsQuery {
            limit: None,
            cursor: Some("not-a-cursor".to_owned()),
        },
    ] {
        let error = list_friends(&setup.state, &setup.alice_access_token, query)
            .await
            .expect_err("invalid pagination should fail");
        assert!(matches!(
            error,
            crate::features::social::SocialError::BadRequest(_)
        ));
    }
}

async fn page(
    setup: &PairSetup,
    cursor: Option<String>,
) -> cheenhub_contracts::rest::ListFriendsResponse {
    list_friends(
        &setup.state,
        &setup.alice_access_token,
        ListFriendsQuery {
            limit: Some(1),
            cursor,
        },
    )
    .await
    .expect("friends page should load")
}

fn last_body(response: &cheenhub_contracts::rest::ListFriendsResponse) -> Option<&str> {
    response.friends[0]
        .last_message
        .as_ref()
        .map(|message| message.body.as_str())
}

async fn befriend_and_open(
    state: &AppState,
    requester_access_token: &str,
    recipient_access_token: &str,
    recipient_user_id: &str,
) -> String {
    let request = send_friend_request(
        state,
        requester_access_token,
        SendFriendRequestRequest {
            recipient_user_id: recipient_user_id.to_owned(),
        },
    )
    .await
    .expect("friend request should send");
    accept_friend_request(state, recipient_access_token, request.request.id)
        .await
        .expect("friend request should accept");
    open_dm_conversation(
        state,
        requester_access_token,
        OpenDmConversationRequest {
            friend_user_id: recipient_user_id.to_owned(),
        },
    )
    .await
    .expect("conversation should open")
    .conversation
    .id
}

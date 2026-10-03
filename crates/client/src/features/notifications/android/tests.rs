use super::*;

#[test]
fn notification_click_targets_direct_message_route() {
    let conversation_id = "80c993e1-2fe7-49e0-bcc5-c56c790d98c8".to_owned();

    assert_eq!(
        direct_message_route(conversation_id.clone()),
        Route::AppDirectMessage { conversation_id }
    );
}

#[test]
fn friend_request_notification_click_targets_friends_route() {
    assert_eq!(friend_requests_route(), Route::AppFriends {});
}

//! Проверки маршрута ЛС до успешной загрузки realtime-обзора.

use super::{DirectMessageRouteAction, direct_message_route_action};

#[test]
fn missing_overview_preserves_saved_route_even_after_retry() {
    assert!(matches!(
        direct_message_route_action("saved", None, false, true),
        DirectMessageRouteAction::Wait
    ));
}

#[test]
fn successful_empty_overview_requests_refresh_before_rejecting_route() {
    assert!(matches!(
        direct_message_route_action("saved", Some(&[]), false, false),
        DirectMessageRouteAction::Refresh
    ));
    assert!(matches!(
        direct_message_route_action("saved", Some(&[]), false, true),
        DirectMessageRouteAction::Unavailable
    ));
}

#[test]
fn loading_overview_does_not_reject_saved_route() {
    assert!(matches!(
        direct_message_route_action("saved", Some(&[]), true, true),
        DirectMessageRouteAction::Wait
    ));
}

#[test]
fn known_conversation_remains_selectable_during_overview_refresh() {
    let overview = [super::DmConversationSummary {
        id: "saved".to_owned(),
        friend_user_id: "friend".to_owned(),
        friend_nickname: "Друг".to_owned(),
        friend_avatar_url: None,
        unread_count: 0,
        last_read_message_id: None,
        last_read_seq: 0,
        last_read_at: None,
        updated_at: "2026-10-24T00:00:00Z".to_owned(),
    }];

    let action = direct_message_route_action("saved", Some(&overview), true, true);

    assert!(matches!(action, DirectMessageRouteAction::Select(0)));
}

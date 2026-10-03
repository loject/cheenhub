use super::{ListFriendsQuery, ListFriendsResponse};

#[test]
fn friends_response_defaults_pagination_from_older_backend() {
    let response: ListFriendsResponse = serde_json::from_str(r#"{"friends":[]}"#)
        .expect("ответ старого backend должен десериализоваться");

    assert!(response.next_cursor.is_none());
    assert!(!response.has_more);
}

#[test]
fn friends_query_deserializes_cursor_and_limit() {
    let query: ListFriendsQuery =
        serde_json::from_str(r#"{"limit":25,"cursor":"opaque-page-position"}"#)
            .expect("query должен десериализоваться");

    assert_eq!(query.limit, Some(25));
    assert_eq!(query.cursor.as_deref(), Some("opaque-page-position"));
}

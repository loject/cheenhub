//! Проверки realtime-запроса объединённого социального списка.

use cheenhub_contracts::realtime::SocialEntries;

use super::*;

#[tokio::test]
async fn social_list_command_preserves_request_id_and_returns_friends() {
    let pair = pair().await;
    let request_id = Uuid::new_v4();

    let envelope = command(
        &pair.state,
        &pair.alice,
        "list_social_entries",
        serde_json::json!({"limit": 1}),
        Some(request_id),
    )
    .await;

    let page: SocialEntries = response(envelope, "social_entries");
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].user_id, pair.bob.id);
    assert!(page.entries[0].conversation.is_some());
    assert!(!page.has_more);
}

#[tokio::test]
async fn social_list_rejects_missing_request_id_and_invalid_limit() {
    let pair = pair().await;
    let missing = command(
        &pair.state,
        &pair.alice,
        "list_social_entries",
        serde_json::json!({}),
        None,
    )
    .await;
    rejection(missing, RejectionCode::BadRequest);

    let invalid = command(
        &pair.state,
        &pair.alice,
        "list_social_entries",
        serde_json::json!({"limit": 0}),
        Some(Uuid::new_v4()),
    )
    .await;
    rejection(invalid, RejectionCode::BadRequest);
}

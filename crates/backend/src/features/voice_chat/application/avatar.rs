//! Voice avatar presence helpers.

use uuid::Uuid;

use crate::state::AppState;

/// Updates active voice presence snapshots after a profile avatar change.
pub(crate) async fn update_user_avatar(
    state: &AppState,
    user_id: &Uuid,
    avatar_url: Option<String>,
) {
    let rooms = state
        .voice_presence_store
        .update_user_avatar(user_id, avatar_url)
        .await;
    if rooms.is_empty() {
        return;
    }

    tracing::info!(
        user_id = %user_id,
        rooms = rooms.len(),
        "updated active voice presence avatar"
    );
    for target in rooms {
        let snapshot = super::room_snapshot(state, target).await;
        super::fanout_snapshot(state, target, snapshot).await;
    }
}

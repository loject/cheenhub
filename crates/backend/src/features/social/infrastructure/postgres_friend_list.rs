//! PostgreSQL-выборка страницы друзей.

use chrono::{DateTime, Utc};
use sea_orm::{DatabaseBackend, DatabaseConnection, FromQueryResult, Statement};
use uuid::Uuid;

use crate::features::social::domain::{
    DmMessage, FriendListCursor, FriendListEntry, Friendship, FriendshipStatus,
};

use super::FriendListPage;

#[derive(Debug, FromQueryResult)]
struct FriendListRow {
    friendship_id: Uuid,
    requester_user_id: Uuid,
    recipient_user_id: Uuid,
    user_low_id: Uuid,
    user_high_id: Uuid,
    friendship_status: String,
    friendship_created_at: DateTime<Utc>,
    friendship_updated_at: DateTime<Utc>,
    friend_user_id: Uuid,
    unread_count: i64,
    message_id: Option<Uuid>,
    conversation_id: Option<Uuid>,
    message_seq: Option<i64>,
    sender_user_id: Option<Uuid>,
    message_body: Option<String>,
    image_id: Option<Uuid>,
    message_created_at: Option<DateTime<Utc>>,
    message_updated_at: Option<DateTime<Utc>>,
    message_deleted_at: Option<DateTime<Utc>>,
}

pub(super) async fn friend_list_page(
    database: &DatabaseConnection,
    user_id: &Uuid,
    cursor: Option<&FriendListCursor>,
    limit: usize,
) -> anyhow::Result<FriendListPage> {
    let cursor_active = cursor.is_some();
    let cursor_has_message = cursor
        .and_then(|cursor| cursor.last_message_created_at)
        .is_some();
    let cursor_created_at = cursor.and_then(|cursor| cursor.last_message_created_at);
    let cursor_friend_user_id = cursor
        .map(|cursor| cursor.friend_user_id)
        .unwrap_or(Uuid::nil());
    let fetch_limit = i64::try_from(limit.saturating_add(1))?;
    // LATERAL выбирает последнее неудалённое сообщение до применения keyset-курсора.
    // CASE-ключ пары не выражается через текущие Entity без relations.
    let statement = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        FRIEND_LIST_PAGE_SQL,
        [
            (*user_id).into(),
            cursor_active.into(),
            cursor_has_message.into(),
            cursor_created_at.into(),
            cursor_friend_user_id.into(),
            fetch_limit.into(),
        ],
    );
    let mut entries = FriendListRow::find_by_statement(statement)
        .all(database)
        .await?
        .into_iter()
        .map(try_friend_list_entry)
        .collect::<anyhow::Result<Vec<_>>>()?;
    let has_more = entries.len() > limit;
    entries.truncate(limit);
    Ok(FriendListPage { entries, has_more })
}

fn try_friend_list_entry(row: FriendListRow) -> anyhow::Result<FriendListEntry> {
    let status = FriendshipStatus::from_str(&row.friendship_status)
        .ok_or_else(|| anyhow::anyhow!("unknown friendship status {}", row.friendship_status))?;
    let last_message = match row.message_id {
        Some(id) => Some(DmMessage {
            id,
            conversation_id: required(row.conversation_id, "conversation_id")?,
            seq: required(row.message_seq, "message_seq")?,
            sender_user_id: required(row.sender_user_id, "sender_user_id")?,
            body: required(row.message_body, "message_body")?,
            image_id: row.image_id,
            created_at: required(row.message_created_at, "message_created_at")?,
            updated_at: required(row.message_updated_at, "message_updated_at")?,
            deleted_at: row.message_deleted_at,
        }),
        None => None,
    };
    Ok(FriendListEntry {
        friendship: Friendship {
            id: row.friendship_id,
            requester_user_id: row.requester_user_id,
            recipient_user_id: row.recipient_user_id,
            user_low_id: row.user_low_id,
            user_high_id: row.user_high_id,
            status,
            created_at: row.friendship_created_at,
            updated_at: row.friendship_updated_at,
        },
        friend_user_id: row.friend_user_id,
        unread_count: row.unread_count,
        last_message,
    })
}

fn required<T>(value: Option<T>, field: &str) -> anyhow::Result<T> {
    value.ok_or_else(|| anyhow::anyhow!("friend list latest message is missing {field}"))
}

const FRIEND_LIST_PAGE_SQL: &str = r#"
SELECT f.id AS friendship_id, f.requester_user_id, f.recipient_user_id,
    f.user_low_id, f.user_high_id, f.status AS friendship_status,
    f.created_at AS friendship_created_at, f.updated_at AS friendship_updated_at,
    CASE WHEN f.user_low_id = $1 THEN f.user_high_id ELSE f.user_low_id END AS friend_user_id,
    COALESCE(cms.unread_count, 0)::bigint AS unread_count,
    lm.id AS message_id, lm.conversation_id, lm.seq AS message_seq, lm.sender_user_id,
    lm.body AS message_body, lm.image_id, lm.created_at AS message_created_at,
    lm.updated_at AS message_updated_at, lm.deleted_at AS message_deleted_at
FROM friendships AS f
LEFT JOIN dm_conversations AS c
    ON c.user_low_id = f.user_low_id AND c.user_high_id = f.user_high_id
LEFT JOIN conversation_member_state AS cms
    ON cms.conversation_id = c.id AND cms.user_id = $1
LEFT JOIN LATERAL (
    SELECT m.* FROM dm_messages AS m
    WHERE m.conversation_id = c.id AND m.deleted_at IS NULL
    ORDER BY m.seq DESC, m.id DESC LIMIT 1
) AS lm ON TRUE
WHERE f.status = 'accepted' AND (f.user_low_id = $1 OR f.user_high_id = $1)
  AND (NOT $2::boolean
    OR ($3::boolean AND (lm.created_at < $4::timestamptz
      OR (lm.created_at = $4::timestamptz
        AND CASE WHEN f.user_low_id = $1 THEN f.user_high_id ELSE f.user_low_id END > $5)
      OR lm.created_at IS NULL))
    OR (NOT $3::boolean AND lm.created_at IS NULL
      AND CASE WHEN f.user_low_id = $1 THEN f.user_high_id ELSE f.user_low_id END > $5))
ORDER BY lm.created_at DESC NULLS LAST, friend_user_id ASC
LIMIT $6
"#;

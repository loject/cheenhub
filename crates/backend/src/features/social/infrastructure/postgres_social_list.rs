//! Ограниченный read-only SQL общего списка друзей и личных собеседников.
//!
//! Единственное пересечение с auth-инфраструктурой — чтение публичного никнейма
//! из `users` для поиска и keyset-порядка до применения размера страницы.
//! надо подумать как избавиться это этого

use chrono::{DateTime, Utc};
use sea_orm::{DatabaseBackend, DatabaseConnection, FromQueryResult, Statement};
use uuid::Uuid;

use crate::features::social::domain::{
    ConversationMemberState, DmConversation, DmMessage, SocialListCursor, SocialListEntry,
};

use super::SocialListPage;

#[derive(Debug, FromQueryResult)]
struct SocialListRow {
    user_id: Uuid,
    nickname_key: String,
    conversation_id: Option<Uuid>,
    conversation_user_low_id: Option<Uuid>,
    conversation_user_high_id: Option<Uuid>,
    conversation_updated_at: Option<DateTime<Utc>>,
    read_last_read_message_id: Option<Uuid>,
    read_last_read_seq: Option<i64>,
    read_last_read_at: Option<DateTime<Utc>>,
    read_unread_count: Option<i64>,
    read_updated_at: Option<DateTime<Utc>>,
    message_id: Option<Uuid>,
    message_seq: Option<i64>,
    message_sender_user_id: Option<Uuid>,
    message_body: Option<String>,
    message_image_id: Option<Uuid>,
    message_created_at: Option<DateTime<Utc>>,
    message_updated_at: Option<DateTime<Utc>>,
    message_deleted_at: Option<DateTime<Utc>>,
}

pub(super) async fn social_list_page(
    database: &DatabaseConnection,
    user_id: &Uuid,
    normalized_query: &str,
    cursor: Option<&SocialListCursor>,
    limit: usize,
) -> anyhow::Result<SocialListPage> {
    let cursor_active = cursor.is_some();
    let cursor_has_message = cursor
        .and_then(|cursor| cursor.last_message_created_at)
        .is_some();
    let cursor_created_at = cursor.and_then(|cursor| cursor.last_message_created_at);
    let cursor_nickname_key = cursor
        .map(|cursor| cursor.nickname_key.as_str())
        .unwrap_or("");
    let cursor_user_id = cursor.map(|cursor| cursor.user_id).unwrap_or(Uuid::nil());
    let fetch_limit = i64::try_from(limit.saturating_add(1))?;
    // Сначала объединяем UUID, затем читаем никнеймы: поиск и сортировка не
    // должны применять limit к отдельным спискам друзей и диалогов.
    let statement = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        SOCIAL_LIST_PAGE_SQL,
        [
            (*user_id).into(),
            normalized_query.to_owned().into(),
            cursor_active.into(),
            cursor_has_message.into(),
            cursor_created_at.into(),
            cursor_nickname_key.to_owned().into(),
            cursor_user_id.into(),
            fetch_limit.into(),
        ],
    );
    let mut entries = SocialListRow::find_by_statement(statement)
        .all(database)
        .await?
        .into_iter()
        .map(|row| try_entry(row, *user_id))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let has_more = entries.len() > limit;
    entries.truncate(limit);
    Ok(SocialListPage { entries, has_more })
}

fn try_entry(row: SocialListRow, current_user_id: Uuid) -> anyhow::Result<SocialListEntry> {
    let conversation = match row.conversation_id {
        Some(id) => Some(DmConversation {
            id,
            user_low_id: required(row.conversation_user_low_id, "conversation_user_low_id")?,
            user_high_id: required(row.conversation_user_high_id, "conversation_user_high_id")?,
            updated_at: required(row.conversation_updated_at, "conversation_updated_at")?,
        }),
        None => None,
    };
    let read_state = match (row.read_last_read_seq, conversation.as_ref()) {
        (Some(seq), Some(conversation)) => Some(ConversationMemberState {
            conversation_id: conversation.id,
            user_id: current_user_id,
            last_read_message_id: row.read_last_read_message_id,
            last_read_seq: seq,
            last_read_at: row.read_last_read_at,
            unread_count: required(row.read_unread_count, "read_unread_count")?,
            updated_at: required(row.read_updated_at, "read_updated_at")?,
        }),
        _ => None,
    };
    let last_message = match row.message_id {
        Some(id) => Some(DmMessage {
            id,
            conversation_id: conversation
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("social list message without conversation"))?
                .id,
            seq: required(row.message_seq, "message_seq")?,
            sender_user_id: required(row.message_sender_user_id, "message_sender_user_id")?,
            body: required(row.message_body, "message_body")?,
            image_id: row.message_image_id,
            created_at: required(row.message_created_at, "message_created_at")?,
            updated_at: required(row.message_updated_at, "message_updated_at")?,
            deleted_at: row.message_deleted_at,
        }),
        None => None,
    };
    Ok(SocialListEntry {
        user_id: row.user_id,
        nickname_key: row.nickname_key,
        conversation,
        read_state,
        last_message,
    })
}

fn required<T>(value: Option<T>, field: &str) -> anyhow::Result<T> {
    value.ok_or_else(|| anyhow::anyhow!("social list row is missing {field}"))
}

const SOCIAL_LIST_PAGE_SQL: &str = r#"
WITH peers AS (
    SELECT CASE WHEN f.user_low_id = $1 THEN f.user_high_id ELSE f.user_low_id END AS user_id
    FROM friendships AS f
    WHERE f.status = 'accepted' AND (f.user_low_id = $1 OR f.user_high_id = $1)
    UNION
    SELECT CASE WHEN c.user_low_id = $1 THEN c.user_high_id ELSE c.user_low_id END AS user_id
    FROM dm_conversations AS c
    WHERE c.user_low_id = $1 OR c.user_high_id = $1
), candidates AS (
    SELECT p.user_id,
        lower(btrim(CASE WHEN u.deletion_requested_at IS NOT NULL
            THEN 'Удалённый пользователь' ELSE u.nickname END)) AS nickname_key,
        c.id AS conversation_id, c.user_low_id AS conversation_user_low_id,
        c.user_high_id AS conversation_user_high_id,
        c.updated_at AS conversation_updated_at,
        cms.last_read_message_id AS read_last_read_message_id,
        cms.last_read_seq AS read_last_read_seq,
        cms.last_read_at AS read_last_read_at,
        cms.unread_count AS read_unread_count,
        cms.updated_at AS read_updated_at,
        lm.id AS message_id, lm.seq AS message_seq,
        lm.sender_user_id AS message_sender_user_id, lm.body AS message_body,
        lm.image_id AS message_image_id, lm.created_at AS message_created_at,
        lm.updated_at AS message_updated_at, lm.deleted_at AS message_deleted_at
    FROM peers AS p
    JOIN users AS u ON u.id = p.user_id AND u.kind = 'user'
    LEFT JOIN dm_conversations AS c ON
        (c.user_low_id = $1 AND c.user_high_id = p.user_id)
        OR (c.user_high_id = $1 AND c.user_low_id = p.user_id)
    LEFT JOIN conversation_member_state AS cms
        ON cms.conversation_id = c.id AND cms.user_id = $1
    LEFT JOIN LATERAL (
        SELECT m.* FROM dm_messages AS m
        WHERE m.conversation_id = c.id AND m.deleted_at IS NULL
        ORDER BY m.seq DESC, m.id DESC LIMIT 1
    ) AS lm ON TRUE
)
SELECT * FROM candidates
WHERE ($2::text = '' OR strpos(nickname_key, $2::text) > 0)
  AND (NOT $3::boolean
    OR ($4::boolean AND (
        message_created_at < $5::timestamptz
        OR (message_created_at = $5::timestamptz
            AND (nickname_key COLLATE "C" > $6::text COLLATE "C"
                OR (nickname_key COLLATE "C" = $6::text COLLATE "C" AND user_id > $7::uuid)))
        OR message_created_at IS NULL))
    OR (NOT $4::boolean AND message_created_at IS NULL
        AND (nickname_key COLLATE "C" > $6::text COLLATE "C"
            OR (nickname_key COLLATE "C" = $6::text COLLATE "C" AND user_id > $7::uuid))))
ORDER BY message_created_at DESC NULLS LAST, nickname_key COLLATE "C" ASC, user_id ASC
LIMIT $8
"#;

//! Вспомогательные функции отображения social-экрана.

use std::time::Duration;

use cheenhub_contracts::realtime::TextChatMessage;
use cheenhub_contracts::rest::{
    DmConversationSummary, DmMessageSummary, FriendRequestSummary, FriendSummary,
    UserRelationStatus,
};
use dioxus::prelude::*;

use crate::features::application_focus::application_is_focused;
use crate::features::runtime::sleep_duration;
use crate::features::text_chat::{ScrollCommand, capture_scroll_position};

use super::api;
use super::direct_message_state::DirectMessageState;

/// Состояние постраничной загрузки списка друзей.
#[derive(Clone, Copy)]
pub(super) struct FriendsPageState {
    /// Уже загруженные друзья.
    pub(super) friends: Signal<Vec<FriendSummary>>,
    /// Курсор следующей страницы.
    pub(super) next_cursor: Signal<Option<String>>,
    /// Есть ли следующая страница.
    pub(super) has_more: Signal<bool>,
    /// Выполняется ли запрос следующей страницы.
    pub(super) is_loading_more: Signal<bool>,
    /// Версия состояния, для которой разрешено применять ответы запросов.
    pub(super) generation: Signal<u64>,
}

pub(super) fn load_social_overview(
    mut friends_page: FriendsPageState,
    mut incoming: Signal<Vec<FriendRequestSummary>>,
    mut outgoing: Signal<Vec<FriendRequestSummary>>,
    mut conversations: Signal<Vec<DmConversationSummary>>,
    mut status: Signal<String>,
    mut is_loading: Signal<bool>,
) {
    let request_generation = (friends_page.generation)().wrapping_add(1);
    friends_page.generation.set(request_generation);
    friends_page.is_loading_more.set(false);
    is_loading.set(true);
    status.set(String::new());
    spawn(async move {
        let result = async {
            let loaded_friends_page = api::list_friends(None).await?;
            let next_incoming = api::list_incoming_requests().await?;
            let next_outgoing = api::list_outgoing_requests().await?;
            let next_conversations = api::list_dm_conversations().await?;
            Ok::<_, String>((
                loaded_friends_page,
                next_incoming,
                next_outgoing,
                next_conversations,
            ))
        }
        .await;

        if (friends_page.generation)() != request_generation {
            debug!(request_generation, "ignored stale social overview response");
            return;
        }

        match result {
            Ok((loaded_friends_page, next_incoming, next_outgoing, next_conversations)) => {
                friends_page.friends.set(loaded_friends_page.friends);
                friends_page
                    .next_cursor
                    .set(loaded_friends_page.next_cursor);
                friends_page.has_more.set(loaded_friends_page.has_more);
                incoming.set(next_incoming);
                outgoing.set(next_outgoing);
                conversations.set(next_conversations);
                status.set(String::new());
            }
            Err(error) => {
                warn!(%error, "failed to load social overview");
                status.set(error);
            }
        }
        is_loading.set(false);
    });
}

pub(super) fn load_more_friends(mut friends_page: FriendsPageState, mut status: Signal<String>) {
    if (friends_page.is_loading_more)() || !(friends_page.has_more)() {
        return;
    }
    let Some(cursor) = (friends_page.next_cursor)() else {
        friends_page.has_more.set(false);
        return;
    };

    let request_generation = (friends_page.generation)();
    friends_page.is_loading_more.set(true);
    spawn(async move {
        let result = api::list_friends(Some(&cursor)).await;
        if (friends_page.generation)() != request_generation {
            debug!(request_generation, "ignored stale friends page response");
            return;
        }
        match result {
            Ok(page) => {
                let mut loaded = (friends_page.friends)();
                for friend in page.friends {
                    if !loaded.iter().any(|saved| saved.user_id == friend.user_id) {
                        loaded.push(friend);
                    }
                }
                let loaded_count = loaded.len();
                friends_page.friends.set(loaded);
                friends_page.next_cursor.set(page.next_cursor);
                friends_page.has_more.set(page.has_more);
                status.set(String::new());
                debug!(loaded_count, "loaded next friends page");
            }
            Err(error) => {
                warn!(%error, "failed to load next friends page");
                status.set(error);
            }
        }
        friends_page.is_loading_more.set(false);
    });
}

pub(super) fn load_messages(
    conversation_id: String,
    mut state: DirectMessageState,
    on_overview_changed: Callback<()>,
) {
    state.is_loading.set(true);
    state.status.set(String::new());
    spawn(async move {
        match api::list_dm_messages(&conversation_id, None).await {
            Ok(response) => {
                state.has_more.set(response.has_more);
                let next_messages = response.messages;
                mark_latest_message_read_if_focused(
                    &conversation_id,
                    &next_messages,
                    on_overview_changed,
                )
                .await;
                state.messages.set(next_messages);
                state.pending_scroll.set(Some(ScrollCommand::Bottom));
            }
            Err(error) => state.status.set(error),
        }
        state.is_loading.set(false);
    });
}

pub(super) fn load_older_messages(conversation_id: String, mut state: DirectMessageState) {
    if (state.is_loading_older)() || !(state.has_more)() {
        return;
    }
    let Some(before_message_id) = (state.messages)().first().map(|message| message.id.clone())
    else {
        return;
    };
    state.is_loading_older.set(true);
    spawn(async move {
        let before_scroll = match state.list_element.cloned() {
            Some(element) => capture_scroll_position(element).await,
            None => None,
        };
        match api::list_dm_messages(&conversation_id, Some(&before_message_id)).await {
            Ok(response) => {
                let mut next = response.messages;
                next.extend((state.messages)());
                next.sort_by(|left, right| left.created_at.cmp(&right.created_at));
                next.dedup_by(|left, right| left.id == right.id);
                state.messages.set(next);
                state.has_more.set(response.has_more);
                if let Some((offset_y, height)) = before_scroll {
                    state
                        .pending_scroll
                        .set(Some(ScrollCommand::Preserve { offset_y, height }));
                }
                debug!(conversation_id, "loaded older direct messages");
            }
            Err(error) => {
                warn!(conversation_id, %error, "failed to load older direct messages");
                state.status.set(error);
            }
        }
        state.is_loading_older.set(false);
    });
}

pub(super) fn refresh_messages(
    conversation_id: String,
    mut state: DirectMessageState,
    on_overview_changed: Callback<()>,
) {
    spawn(async move {
        let oldest_loaded_id = (state.messages)().first().map(|message| message.id.clone());
        match fetch_loaded_dm_messages(&conversation_id, oldest_loaded_id.as_deref()).await {
            Ok(next_messages) => {
                debug!(
                    conversation_id = %conversation_id,
                    message_count = next_messages.len(),
                    "refreshed direct messages"
                );
                set_messages_with_motion(
                    state.messages,
                    state.appearing_message_ids,
                    state.removing_message_ids,
                    next_messages,
                );
                if (state.is_near_bottom)() && application_is_focused() {
                    mark_latest_message_read(
                        &conversation_id,
                        &(state.messages)(),
                        on_overview_changed,
                    )
                    .await;
                    state.pending_scroll.set(Some(ScrollCommand::Bottom));
                } else if (state.is_near_bottom)() {
                    debug!(
                        conversation_id = %conversation_id,
                        "preserved direct message unread state while application is unfocused"
                    );
                }
            }
            Err(error) => {
                warn!(conversation_id = %conversation_id, %error, "failed to refresh direct messages");
                state.status.set(error);
            }
        }
    });
}

pub(super) fn push_message_with_motion(
    mut messages: Signal<Vec<DmMessageSummary>>,
    mut appearing_message_ids: Signal<Vec<String>>,
    message: DmMessageSummary,
) -> bool {
    if messages()
        .iter()
        .any(|saved_message| saved_message.id == message.id)
    {
        return false;
    }
    let message_id = message.id.clone();
    messages.write().push(message);
    appearing_message_ids.write().push(message_id.clone());
    clear_appearing_after(appearing_message_ids, message_id);

    true
}

/// Перечитывает все открытые страницы истории: realtime-событие не содержит ID правки.
async fn fetch_loaded_dm_messages(
    conversation_id: &str,
    oldest_loaded_id: Option<&str>,
) -> Result<Vec<DmMessageSummary>, String> {
    let mut response = api::list_dm_messages(conversation_id, None).await?;
    let mut latest = response.messages;
    while response.has_more
        && oldest_loaded_id.is_some_and(|oldest| !latest.iter().any(|message| message.id == oldest))
    {
        let Some(cursor) = latest.first().map(|message| message.id.clone()) else {
            break;
        };
        response = api::list_dm_messages(conversation_id, Some(&cursor)).await?;
        if response.messages.is_empty() {
            break;
        }
        let mut older = response.messages;
        older.extend(latest);
        latest = older;
    }
    Ok(latest)
}

fn set_messages_with_motion(
    mut messages: Signal<Vec<DmMessageSummary>>,
    mut appearing_message_ids: Signal<Vec<String>>,
    _removing_message_ids: Signal<Vec<String>>,
    next_messages: Vec<DmMessageSummary>,
) {
    let (merged, added_ids) = merge_direct_messages(messages(), next_messages);
    for added_id in added_ids {
        appearing_message_ids.write().push(added_id.clone());
        clear_appearing_after(appearing_message_ids, added_id);
    }
    messages.set(merged);
}

/// Обновляет совпадающие сообщения, не теряя ранее загруженные страницы истории.
fn merge_direct_messages(
    mut previous: Vec<DmMessageSummary>,
    incoming: Vec<DmMessageSummary>,
) -> (Vec<DmMessageSummary>, Vec<String>) {
    let mut added_ids = Vec::new();
    for next in incoming {
        if let Some(existing) = previous.iter_mut().find(|item| item.id == next.id) {
            *existing = next;
        } else {
            added_ids.push(next.id.clone());
            previous.push(next);
        }
    }
    previous.sort_by_key(|message| message.seq);
    (previous, added_ids)
}

fn clear_appearing_after(mut appearing_message_ids: Signal<Vec<String>>, message_id: String) {
    spawn(async move {
        sleep_duration(Duration::from_millis(220)).await;
        appearing_message_ids
            .write()
            .retain(|appearing_id| appearing_id != &message_id);
    });
}

pub(super) fn dm_as_text_message(message: DmMessageSummary) -> TextChatMessage {
    TextChatMessage {
        id: message.id,
        server_id: String::new(),
        room_id: message.conversation_id.clone(),
        author_user_id: message.sender_user_id,
        author_nickname: message.sender_nickname,
        author_avatar_url: message.sender_avatar_url,
        body: message.body,
        attachments: Vec::new(),
        delivery_status: message.delivery_status,
        created_at: message.created_at,
        edited_at: message.edited_at,
    }
}

pub(super) fn relation_label(relation: Option<UserRelationStatus>) -> &'static str {
    match relation {
        Some(UserRelationStatus::Friends) => "Уже в друзьях",
        Some(UserRelationStatus::PendingOutgoing) => "Заявка отправлена",
        Some(UserRelationStatus::PendingIncoming) => "Ждёт вашего ответа",
        None => "Можно добавить",
    }
}

async fn mark_latest_message_read_if_focused(
    conversation_id: &str,
    messages: &[DmMessageSummary],
    on_overview_changed: Callback<()>,
) {
    if !application_is_focused() {
        debug!(
            conversation_id,
            "preserved direct message unread state while application is unfocused"
        );
        return;
    }
    mark_latest_message_read(conversation_id, messages, on_overview_changed).await;
}

async fn mark_latest_message_read(
    conversation_id: &str,
    messages: &[DmMessageSummary],
    on_overview_changed: Callback<()>,
) {
    let Some(last_message) = messages.last() else {
        return;
    };
    match api::mark_dm_conversation_read(conversation_id, last_message.id.clone()).await {
        Ok(read_update) => {
            debug!(
                conversation_id = %conversation_id,
                conversation_unread_count = read_update.conversation_unread_count,
                total_unread_count = read_update.total_unread_count,
                "marked direct conversation read"
            );
            on_overview_changed.call(());
        }
        Err(error) => {
            warn!(conversation_id = %conversation_id, %error, "failed to mark direct conversation read")
        }
    }
}

#[cfg(test)]
mod tests;

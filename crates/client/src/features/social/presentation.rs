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

pub(super) fn load_social_overview(
    mut friends: Signal<Vec<FriendSummary>>,
    mut incoming: Signal<Vec<FriendRequestSummary>>,
    mut outgoing: Signal<Vec<FriendRequestSummary>>,
    mut conversations: Signal<Vec<DmConversationSummary>>,
    mut status: Signal<String>,
    mut is_loading: Signal<bool>,
) {
    is_loading.set(true);
    status.set(String::new());
    spawn(async move {
        let result = async {
            let next_friends = api::list_friends().await?;
            let next_incoming = api::list_incoming_requests().await?;
            let next_outgoing = api::list_outgoing_requests().await?;
            let next_conversations = api::list_dm_conversations().await?;
            Ok::<_, String>((
                next_friends,
                next_incoming,
                next_outgoing,
                next_conversations,
            ))
        }
        .await;

        match result {
            Ok((next_friends, next_incoming, next_outgoing, next_conversations)) => {
                friends.set(next_friends);
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
        match api::list_dm_messages(&conversation_id, None).await {
            Ok(response) => {
                let next_messages = response.messages;
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

fn set_messages_with_motion(
    mut messages: Signal<Vec<DmMessageSummary>>,
    mut appearing_message_ids: Signal<Vec<String>>,
    mut removing_message_ids: Signal<Vec<String>>,
    next_messages: Vec<DmMessageSummary>,
) {
    let previous_messages = messages();
    let added_ids = next_messages
        .iter()
        .filter(|message| {
            !previous_messages
                .iter()
                .any(|previous| previous.id == message.id)
        })
        .map(|message| message.id.clone())
        .collect::<Vec<_>>();
    let removed_ids = previous_messages
        .iter()
        .filter(|message| {
            !next_messages
                .iter()
                .any(|next_message| next_message.id == message.id)
        })
        .map(|message| message.id.clone())
        .collect::<Vec<_>>();

    removing_message_ids.write().retain(|message_id| {
        !next_messages
            .iter()
            .any(|message| message.id == *message_id)
    });

    if added_ids.is_empty() && removed_ids.is_empty() {
        messages.set(next_messages);
        return;
    }

    let mut combined_messages = Vec::new();
    for previous in &previous_messages {
        if let Some(next) = next_messages
            .iter()
            .find(|next_message| next_message.id == previous.id)
        {
            combined_messages.push(next.clone());
        } else {
            combined_messages.push(previous.clone());
        }
    }
    for next in next_messages {
        if !combined_messages
            .iter()
            .any(|message| message.id == next.id)
        {
            combined_messages.push(next);
        }
    }

    for added_id in &added_ids {
        appearing_message_ids.write().push(added_id.clone());
        clear_appearing_after(appearing_message_ids, added_id.clone());
    }
    for removed_id in &removed_ids {
        removing_message_ids.write().push(removed_id.clone());
    }
    messages.set(combined_messages);

    if !removed_ids.is_empty() {
        spawn(async move {
            sleep_duration(Duration::from_millis(180)).await;
            let active_removed_ids = {
                let current_removing_ids = removing_message_ids();
                removed_ids
                    .iter()
                    .filter(|message_id| current_removing_ids.contains(message_id))
                    .cloned()
                    .collect::<Vec<_>>()
            };
            if active_removed_ids.is_empty() {
                return;
            }
            messages
                .write()
                .retain(|message| !active_removed_ids.contains(&message.id));
            removing_message_ids
                .write()
                .retain(|message_id| !active_removed_ids.contains(message_id));
        });
    }
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

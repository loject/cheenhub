//! Группа последовательных личных сообщений одного автора.

use cheenhub_contracts::rest::DmMessageSummary;
use dioxus::prelude::*;

use crate::features::app::components::avatar::{UserAvatar, use_avatar_seed};
use crate::features::app::current_user::CurrentUserContext;
use crate::features::text_chat::{
    ChatMessageItem, estimated_group_height, estimated_image_preview_height, friendly_message_date,
    is_appearing_message, message_day_key,
};

use super::direct_message_image::DirectMessageImage;
use super::presentation::dm_as_text_message;

pub(super) type VirtualDirectMessageGroup = (String, Option<String>, f64, Vec<DmMessageSummary>);

/// Группирует соседние DM и рассчитывает метаданные виртуальной строки за один проход.
pub(super) fn prepare_direct_message_groups(
    messages: &[DmMessageSummary],
) -> Vec<VirtualDirectMessageGroup> {
    let mut groups = Vec::<Vec<DmMessageSummary>>::new();
    for message in messages {
        match groups.last_mut() {
            Some(group)
                if group.last().is_some_and(|last| {
                    last.sender_user_id == message.sender_user_id
                        && message_day_key(&last.created_at) == message_day_key(&message.created_at)
                }) =>
            {
                group.push(message.clone());
            }
            _ => groups.push(vec![message.clone()]),
        }
    }

    let mut previous_day_key = None;
    groups
        .into_iter()
        .filter_map(|group| {
            let first_message = group.first()?;
            let day_key = message_day_key(&first_message.created_at);
            let date_label = (previous_day_key.as_ref() != Some(&day_key))
                .then(|| friendly_message_date(&first_message.created_at));
            previous_day_key = Some(day_key);
            let estimated_height = estimated_group_height(
                group.iter().map(|message| message.body.chars().count()),
                group.iter().filter_map(|message| {
                    message
                        .image
                        .as_ref()
                        .map(|image| estimated_image_preview_height(image.width, image.height))
                }),
            );
            Some((
                first_message.id.clone(),
                date_label,
                estimated_height,
                group,
            ))
        })
        .collect()
}

/// Рендерит сообщения и изображения в исходном порядке внутри авторской группы.
#[component]
pub(super) fn DirectMessageGroup(
    messages: Vec<DmMessageSummary>,
    appearing_message_ids: Vec<String>,
    removing_message_ids: Vec<String>,
) -> Element {
    let Some(first_message) = messages.first().cloned() else {
        return rsx! {};
    };

    use_avatar_seed(first_message.sender_user_id.clone());
    let current_user = use_context::<CurrentUserContext>().require_user();
    let is_own_group = first_message.sender_user_id == current_user.id;
    let avatar_column_style = if is_own_group {
        "grid-column: 3; grid-row: 1;"
    } else {
        "grid-column: 1; grid-row: 1;"
    };
    let header_class = if is_own_group {
        "mb-1 flex items-center justify-end gap-2"
    } else {
        "mb-1 flex items-center gap-2"
    };

    rsx! {
        div {
            class: "chat-message-group relative grid gap-3",
            style: "grid-template-columns: 2.25rem minmax(0, 1fr) 2.25rem;",
            div {
                class: "chat-message-avatar-column sticky top-0 z-20 shrink-0 self-start pt-1",
                style: avatar_column_style,
                UserAvatar {
                    nickname: first_message.sender_nickname.clone(),
                    avatar_url: first_message.sender_avatar_url.clone(),
                    class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100".to_owned(),
                    avatar_seed: Some(first_message.sender_user_id.clone()),
                }
            }
            div {
                class: "chat-message-content min-w-0",
                style: "grid-column: 2; grid-row: 1;",
                div { class: header_class,
                    span { class: "truncate text-[12px] font-semibold text-zinc-100", "{first_message.sender_nickname}" }
                }
                div { class: "flex flex-col gap-2",
                    for message in messages.iter().cloned() {
                        ChatMessageItem {
                            key: "{message.id}",
                            message: dm_as_text_message(message.clone()),
                            animate: is_appearing_message(&message.id, &appearing_message_ids),
                            removing: removing_message_ids.contains(&message.id),
                            can_delete_messages: false,
                            on_delete: move |_| {},
                            if let Some(image) = message.image.clone() {
                                DirectMessageImage {
                                    conversation_id: message.conversation_id.clone(),
                                    author_user_id: message.sender_user_id.clone(),
                                    image,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use cheenhub_contracts::rest::{DmImageAttachmentSummary, DmMessageSummary};

    use super::prepare_direct_message_groups;

    fn message(id: &str, sender: &str, created_at: &str) -> DmMessageSummary {
        DmMessageSummary {
            id: id.to_owned(),
            conversation_id: "conversation".to_owned(),
            seq: id.parse().unwrap_or_default(),
            sender_user_id: sender.to_owned(),
            sender_nickname: sender.to_owned(),
            sender_avatar_url: None,
            body: "текст".to_owned(),
            image: None,
            delivery_status: None,
            created_at: created_at.to_owned(),
        }
    }

    #[test]
    fn virtual_dm_groups_preserve_order_and_split_on_author_or_day() {
        let messages = [
            message("1", "alice", "2025-07-12T08:00:00Z"),
            message("2", "alice", "2025-07-12T09:00:00Z"),
            message("3", "bob", "2025-07-12T10:00:00Z"),
            message("4", "bob", "2025-07-13T10:00:00Z"),
        ];

        let groups = prepare_direct_message_groups(&messages);

        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].0, "1");
        assert_eq!(
            groups[0]
                .3
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["1", "2"]
        );
        assert_eq!(groups[1].0, "3");
        assert_eq!(groups[2].0, "4");
        assert!(groups[0].1.is_some());
        assert!(groups[1].1.is_none());
        assert!(groups[2].1.is_some());
    }

    #[test]
    fn virtual_dm_group_estimate_accounts_for_an_image() {
        let plain = prepare_direct_message_groups(&[message("1", "alice", "2025-07-12T08:00:00Z")]);
        let mut rich_message = message("2", "alice", "2025-07-12T08:00:00Z");
        rich_message.image = Some(DmImageAttachmentSummary {
            id: "image".to_owned(),
            content_type: "image/png".to_owned(),
            width: 1_600,
            height: 900,
        });
        let rich = prepare_direct_message_groups(&[rich_message]);

        assert!(rich[0].2 > plain[0].2 + 290.0);
    }
}

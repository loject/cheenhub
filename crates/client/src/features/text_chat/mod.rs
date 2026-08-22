//! Text chat client feature.

mod attachment_preview;
mod clipboard;
mod compose;
mod compose_actions;
mod history;
mod image_attachment;
mod message_date;
mod message_date_divider;
mod message_group;
mod message_item;
mod messages;
mod panel;
mod pending_attachment;
pub(crate) mod realtime;
mod room_compose_state;
mod scroll;
mod surface;

/// Общая ширина визуальной группы формы и ожидающего вложения.
pub(crate) const CHAT_COMPOSER_GROUP_CLASS: &str = "mx-auto min-w-0 w-full max-w-5xl space-y-2";
/// Оформление формы ввода сообщений.
pub(crate) const CHAT_COMPOSER_CLASS: &str = concat!(
    "chat-input-wrap flex min-w-0 w-full items-end gap-2 rounded-[20px] ",
    "border border-zinc-800 bg-[rgba(39,39,42,.8)] p-2 ",
    "shadow-[0_0_0_1px_rgba(255,255,255,0.02)]"
);
/// Общая ширина списка сообщений.
pub(crate) const CHAT_CONTENT_CLASS: &str = "mx-auto flex w-full max-w-5xl flex-col gap-4";
pub(crate) use attachment_preview::ChatAttachmentPreview;
pub(crate) use message_date::{friendly_message_date, message_day_key};
pub(crate) use message_date_divider::ChatMessageDateDivider;
pub(crate) use message_group::ChatMessageGroup;
pub(crate) use message_item::ChatMessageItem;
pub(crate) use messages::{group_consecutive_messages, is_appearing_message};
pub(crate) use room_compose_state::{RoomComposeState, use_room_compose_state};
pub(crate) use scroll::{
    ScrollCommand, apply_scroll_command, capture_scroll_position, update_near_bottom_state,
};
pub(crate) use surface::{RoomChatSurface, RoomChatSurfaceMode};

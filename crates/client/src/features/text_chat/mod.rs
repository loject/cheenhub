//! Text chat client feature.

mod compose;
mod history;
mod image_attachment;
mod message_date;
mod message_date_divider;
mod message_group;
mod message_item;
mod messages;
mod panel;
pub(crate) mod realtime;
mod scroll;
mod surface;

/// Общая ширина и оформление формы ввода сообщений.
pub(crate) const CHAT_COMPOSER_CLASS: &str = concat!(
    "chat-input-wrap mx-auto flex w-full max-w-5xl items-end gap-2 rounded-[20px] ",
    "border border-zinc-800 bg-[rgba(39,39,42,.8)] p-2 ",
    "shadow-[0_0_0_1px_rgba(255,255,255,0.02)]"
);
/// Общая ширина списка сообщений.
pub(crate) const CHAT_CONTENT_CLASS: &str = "mx-auto flex w-full max-w-5xl flex-col gap-4";
/// Общая ширина строки статуса текстового чата.
pub(crate) const CHAT_STATUS_CLASS: &str =
    "mx-auto w-full max-w-5xl px-4 pb-2 text-[11px] leading-4 text-red-200";

pub(crate) use message_date::{friendly_message_date, message_day_key};
pub(crate) use message_date_divider::ChatMessageDateDivider;
pub(crate) use message_group::ChatMessageGroup;
pub(crate) use message_item::ChatMessageItem;
pub(crate) use messages::{group_consecutive_messages, is_appearing_message};
pub(crate) use scroll::{
    ScrollCommand, apply_scroll_command, capture_scroll_position, update_near_bottom_state,
};
pub(crate) use surface::{RoomChatSurface, RoomChatSurfaceMode};

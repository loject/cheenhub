//! Text chat client feature.

mod compose;
mod history;
mod history_loading_state;
mod history_viewport;
mod image_attachment;
mod message_date;
mod message_date_divider;
mod message_group;
mod message_item;
mod messages;
mod panel;
pub(crate) mod read_only_notice;
pub(crate) mod realtime;
mod scroll;
mod surface;
mod virtual_list;

/// Общая ширина списка сообщений.
pub(crate) const CHAT_CONTENT_CLASS: &str = "mx-auto flex min-w-0 w-full max-w-5xl flex-col gap-6";
pub(crate) use history_loading_state::ChatHistoryLoadingState;
pub(crate) use history_viewport::use_history_overflow;
pub(crate) use message_date::{friendly_message_date, full_message_datetime, message_day_key};
pub(crate) use message_date_divider::ChatMessageDateDivider;
pub(crate) use message_group::ChatMessageGroup;
pub(crate) use message_item::{ChatMessageItem, message_time};
pub(crate) use messages::{group_consecutive_messages, is_appearing_message};
pub(crate) use scroll::{
    ScrollCommand, apply_scroll_command, capture_scroll_position, update_near_bottom_state,
};
pub(crate) use surface::{RoomChatSurface, RoomChatSurfaceMode};
pub(crate) use virtual_list::{
    VirtualChatLayout, VirtualChatRow, estimated_group_height, estimated_image_preview_height,
    prepare_text_chat_groups,
};

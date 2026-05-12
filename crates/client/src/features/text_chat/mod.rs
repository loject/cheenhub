//! Text chat client feature.

mod compose;
mod history;
mod message_item;
mod messages;
mod panel;
mod realtime;
mod scroll;
mod surface;

pub(crate) use surface::{RoomChatSurface, RoomChatSurfaceMode};

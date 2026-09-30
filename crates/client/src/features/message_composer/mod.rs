//! Общая форма набора и отправки сообщений в комнатах и личных диалогах.

mod attachment_preview;
mod backend;
mod clipboard;
mod component;
mod emoji_catalog;
mod emoji_picker;
pub(crate) mod pending_attachment;
mod sending;
mod state;

pub(crate) use backend::MessageOperations;
pub(crate) use component::MessageComposer;
pub(crate) use state::{MessageComposeState, use_message_compose_state};

/// Общий лимит изображения в форме сообщения.
pub(crate) const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;

#[cfg(test)]
mod sending_lifecycle_tests;

//! Уведомления о новых сообщениях.

mod android;
mod direct_messages;
mod focus;
mod native;
mod unsupported;
mod web;

pub(crate) use focus::ApplicationFocusContext;
pub(crate) use native::{NotificationsProvider, application_is_focused};

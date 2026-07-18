//! Уведомления о новых сообщениях.

mod android;
mod direct_messages;
mod native;
mod unsupported;
mod web;

pub(crate) use native::NotificationsProvider;

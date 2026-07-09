//! Уведомления о новых сообщениях.

mod native;
mod unsupported;
mod web;

pub(crate) use native::NotificationsProvider;

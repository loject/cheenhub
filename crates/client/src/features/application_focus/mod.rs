//! Глобальное состояние фокуса приложения.

mod native;
mod provider;
mod unsupported;
mod web;

pub(crate) use native::application_is_focused;
pub(crate) use provider::{ApplicationFocusContext, ApplicationFocusProvider};

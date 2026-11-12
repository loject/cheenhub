//! Клиентские операции приватности и блокировки пользователей.

pub(crate) mod api;
mod block_button;
mod block_modal;

pub(crate) use block_button::PrivacyBlockButton;
pub(crate) use block_modal::PrivacyBlockModal;

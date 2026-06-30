//! Клиентская часть друзей и личных сообщений.

pub(crate) mod api;
mod friend_context_menu;
mod friend_search_modal;
mod friends_section;
mod page;
mod presentation;
mod realtime;
mod requests_section;

pub(crate) use page::SocialPage;

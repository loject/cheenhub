//! Клиентская часть друзей и личных сообщений.

pub(crate) mod api;
mod direct_message_voice_button;
mod direct_message_voice_surface;
mod direct_message_workspace;
mod friend_context_menu;
mod friend_search_modal;
mod friends_section;
mod page;
mod presentation;
mod realtime;
mod requests_section;
mod voice_target;

pub(crate) use page::SocialPage;

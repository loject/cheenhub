//! Клиентская часть друзей и личных сообщений.

pub(crate) mod api;
mod clipboard;
mod direct_message_composer;
mod direct_message_group;
mod direct_message_image;
mod direct_message_state;
mod direct_message_voice_button;
mod direct_message_voice_surface;
mod direct_message_workspace;
mod friend_context_menu;
mod friend_search_modal;
mod friends_section;
mod page;
mod presentation;
pub(crate) mod realtime;
mod requests_section;
mod voice_target;

pub(crate) use page::SocialPage;

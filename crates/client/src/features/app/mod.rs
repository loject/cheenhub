//! Authenticated application feature.

pub(crate) mod api;
pub(crate) mod components;
pub(crate) mod current_user;
mod pages;

pub(crate) use pages::app_page::AppPage;
pub(crate) use pages::invite_page::InvitePage;

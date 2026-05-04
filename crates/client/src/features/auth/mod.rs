//! Authentication UI feature for the CheenHub web client.

mod behavior;
mod components;
mod domain;
mod pages;

pub(crate) use pages::login_page::LoginPage;
pub(crate) use pages::register_page::RegisterPage;

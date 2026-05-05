//! Authentication UI feature for the CheenHub web client.

pub(crate) mod api;
mod components;
mod domain;
pub(crate) mod jwt;
mod pages;
mod storage;

pub(crate) use pages::login_page::LoginPage;
pub(crate) use pages::register_page::RegisterPage;

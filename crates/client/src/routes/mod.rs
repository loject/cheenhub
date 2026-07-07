//! Компоненты верхнего уровня маршрутов.

mod app_direct_message;
mod app_friends;
mod app_home;
mod app_server;
mod app_server_room;
mod forgot_password;
mod invite;
mod landing;
mod login;
mod not_found;
mod oauth_callback;
mod register;
mod reset_password;

pub(crate) use app_direct_message::AppDirectMessage;
pub(crate) use app_friends::AppFriends;
pub(crate) use app_home::AppHome;
pub(crate) use app_server::AppServer;
pub(crate) use app_server_room::AppServerRoom;
pub(crate) use forgot_password::ForgotPassword;
pub(crate) use invite::Invite;
pub(crate) use landing::Landing;
pub(crate) use login::Login;
pub(crate) use not_found::NotFound;
pub(crate) use oauth_callback::OAuthCallback;
pub(crate) use register::Register;
pub(crate) use reset_password::ResetPassword;

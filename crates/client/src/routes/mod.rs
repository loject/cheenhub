//! Top-level route components.

mod app_home;
mod forgot_password;
mod invite;
mod landing;
mod login;
mod oauth_callback;
mod register;
mod reset_password;

pub(crate) use app_home::AppHome;
pub(crate) use forgot_password::ForgotPassword;
pub(crate) use invite::Invite;
pub(crate) use landing::Landing;
pub(crate) use login::Login;
pub(crate) use oauth_callback::OAuthCallback;
pub(crate) use register::Register;
pub(crate) use reset_password::ResetPassword;

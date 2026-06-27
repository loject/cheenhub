#![warn(missing_docs)]
//! Точка входа web-клиента CheenHub.

use dioxus::prelude::*;

mod features;
mod routes;

use routes::{
    AppHome, ForgotPassword, Invite, Landing, Login, NotFound, OAuthCallback, Register,
    ResetPassword,
};

use crate::features::system_tray::SystemTrayProvider;
use crate::features::toast::ToastProvider;

static TAILWIND_CSS: Asset = asset!(
    "/assets/tailwind.css",
    AssetOptions::css().with_static_head(true)
);

fn main() {
    dioxus_sdk_storage::set_dir!();
    features::runtime::launch_client(App);
}

#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[route("/")]
    Landing {},
    #[route("/login")]
    Login {},
    #[route("/register")]
    Register {},
    #[route("/forgot-password")]
    ForgotPassword {},
    #[route("/reset-password?:token")]
    ResetPassword { token: Option<String> },
    #[route("/auth/oauth/google?:code&:handoff_code&:error")]
    OAuthCallback {
        code: Option<String>,
        handoff_code: Option<String>,
        error: Option<String>,
    },
    #[route("/app")]
    AppHome {},
    #[route("/invite/:code")]
    Invite { code: String },
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        SystemTrayProvider {
            ToastProvider {
                Router::<Route> {}
            }
        }
    }
}

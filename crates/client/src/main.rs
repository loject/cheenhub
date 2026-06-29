#![warn(missing_docs)]
//! Точка входа web-клиента CheenHub.

use dioxus::prelude::*;

mod features;
mod routes;
mod update_mode;

use routes::{
    AppHome, ForgotPassword, Invite, Landing, Login, NotFound, OAuthCallback, Register,
    ResetPassword,
};

use crate::features::application_update::ApplicationUpdateProvider;
use crate::features::pwa::PwaVersionBridge;
use crate::features::system_tray::SystemTrayProvider;

static TAILWIND_CSS: Asset = asset!(
    "/assets/tailwind.css",
    AssetOptions::css().with_static_head(true)
);

fn main() {
    if update_mode::run_if_requested() {
        return;
    }

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
        PwaVersionBridge {}
        SystemTrayProvider {
            ApplicationUpdateProvider {
                Router::<Route> {}
            }
        }
    }
}

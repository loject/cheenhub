#![warn(missing_docs)]
//! Точка входа клиента CheenHub.

use dioxus::prelude::*;

mod features;
mod routes;
mod update_mode;

use routes::{
    AppDirectMessage, AppFriends, AppHome, AppServer, AppServerRoom, ForgotPassword, Invite,
    Landing, Login, NotFound, OAuthCallback, Register, ResetPassword,
};

use crate::features::application_focus::ApplicationFocusProvider;
use crate::features::application_update::ApplicationUpdateProvider;
use crate::features::autostart::AutostartProvider;
use crate::features::pwa::PwaVersionBridge;
use crate::features::single_instance::SingleInstanceEffects;
use crate::features::system_tray::SystemTrayProvider;

static TAILWIND_CSS: Asset = asset!(
    "/assets/tailwind.css",
    AssetOptions::css().with_static_head(true)
);

fn configure_storage() {
    #[cfg(target_os = "android")]
    {
        dioxus_sdk_storage::set_directory(std::path::PathBuf::from(
            "/data/user/0/ru.cheenhub/files/storage",
        ));
    }

    #[cfg(all(not(target_os = "android"), not(target_family = "wasm")))]
    {
        dioxus_sdk_storage::set_dir!();
    }
}

fn main() {
    if update_mode::run_if_requested() {
        return;
    }

    match features::single_instance::prepare() {
        Ok(true) => {}
        Ok(false) => return,
        Err(error) => {
            error!(error = %error, "failed to prepare CheenHub single-instance runtime");
            eprintln!("Не удалось подготовить запуск CheenHub: {error}");
            return;
        }
    }

    configure_storage();
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
    #[nest("/app")]
    #[layout(crate::features::app::AppPage)]
    #[route("/")]
    AppHome {},
    #[route("/friends")]
    AppFriends {},
    #[route("/friends/dm/:conversation_id")]
    AppDirectMessage { conversation_id: String },
    #[route("/servers/:server_id")]
    AppServer { server_id: String },
    #[route("/servers/:server_id/rooms/:room_id")]
    AppServerRoom { server_id: String, room_id: String },
    #[end_layout]
    #[end_nest]
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
        SingleInstanceEffects {}
        AutostartProvider {
            SystemTrayProvider {
                ApplicationFocusProvider {
                    ApplicationUpdateProvider {
                        Router::<Route> {}
                    }
                }
            }
        }
    }
}

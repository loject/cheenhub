#![warn(missing_docs)]
//! Web client entrypoint for CheenHub.

use dioxus::prelude::*;

mod features;
mod routes;

use routes::{AppHome, Landing, Login, Register};

static TAILWIND_CSS: Asset = asset!(
    "/assets/tailwind.css",
    AssetOptions::css().with_static_head(true)
);

fn main() {
    dioxus::launch(App);
}

#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[route("/")]
    Landing {},
    #[route("/login")]
    Login {},
    #[route("/register")]
    Register {},
    #[route("/app")]
    AppHome {},
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        Router::<Route> {}
    }
}

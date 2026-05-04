#![warn(missing_docs)]
//! Web client entrypoint for CheenHub.

use dioxus::prelude::*;

mod features;

use features::landing::LandingPage;

static TAILWIND_CSS: Asset = asset!(
    "/assets/tailwind.css",
    AssetOptions::css().with_static_head(true)
);

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        LandingPage {}
    }
}

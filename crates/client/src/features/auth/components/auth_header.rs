//! Authentication page header.

use dioxus::prelude::*;

use crate::Route;
use crate::features::auth::components::arrow_left_icon::ArrowLeftIcon;

#[component]
pub(crate) fn AuthHeader() -> Element {
    rsx! {
        header { class: "border-b border-zinc-800/80 bg-zinc-950/85 backdrop-blur-xl",
            div { class: "mx-auto flex w-full max-w-6xl items-center justify-between px-5 py-3 lg:px-8",
                Link {
                    to: Route::Landing {},
                    class: "btn-g inline-flex items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-[12px] font-medium text-zinc-300",
                    ArrowLeftIcon { class_name: "h-4 w-4" }
                    "На главную"
                }
                div { class: "flex items-center gap-2 text-[12px] text-zinc-500",
                    span { class: "hidden sm:inline", "CheenHub" }
                    span { class: "h-1.5 w-1.5 rounded-full bg-accent/70" }
                    span { "Вход" }
                }
            }
        }
    }
}

//! External authentication provider button component.

use dioxus::prelude::*;

use crate::features::auth::behavior::show_todo_alert;
use crate::features::auth::components::arrow_right_icon::ArrowRightIcon;
use crate::features::auth::domain::AuthProvider;

#[component]
pub(crate) fn ProviderButton(provider: AuthProvider) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "btn-g flex h-11 w-full items-center justify-between rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[13px] font-medium text-zinc-300",
            onclick: move |_| show_todo_alert(),
            span { class: "flex items-center gap-3",
                span { class: "flex h-7 w-7 items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900 font-semibold text-zinc-200",
                    "{provider.badge()}"
                }
                "{provider.label()}"
            }
            ArrowRightIcon { class_name: "h-4 w-4 text-zinc-600" }
        }
    }
}

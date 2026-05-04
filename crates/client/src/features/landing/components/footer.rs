//! Landing page footer component.

use dioxus::prelude::*;

use crate::features::landing::components::logo_icon::LogoIcon;
use crate::features::landing::components::todo_button::TodoButton;

#[component]
pub(crate) fn Footer() -> Element {
    rsx! {
        footer { class: "border-t border-zinc-800/80 bg-zinc-950/80",
            div { class: "mx-auto flex max-w-6xl flex-col items-center justify-between gap-3 px-5 py-6 text-[12px] text-zinc-600 sm:flex-row lg:px-8",
                div { class: "flex items-center gap-2",
                    div { class: "flex h-6 w-6 items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900 text-zinc-500",
                        LogoIcon { class_name: "h-3.5 w-3.5" }
                    }
                    "CheenHub — лицензия MIT"
                }
                div { class: "flex flex-wrap items-center justify-center gap-4 sm:justify-end",
                    a { href: "https://github.com/loject/cheenhub", target: "_blank", rel: "noopener", class: "transition hover:text-zinc-400", "GitHub" }
                    a { href: "https://youtube.com/@cheenhub", target: "_blank", rel: "noopener", class: "transition hover:text-zinc-400", "YouTube" }
                    TodoButton { class_name: "bg-transparent p-0 transition hover:text-zinc-400", label: "RuTube" }
                    TodoButton { class_name: "bg-transparent p-0 transition hover:text-zinc-400", label: "Telegram" }
                    span { class: "text-zinc-800", "·" }
                    span { "Разработка публично" }
                }
            }
        }
    }
}

//! Landing page call-to-action section component.

use dioxus::prelude::*;

use crate::features::landing::components::download_dropdown::DownloadDropdown;
use crate::features::landing::components::social_links::SocialLinks;
use crate::features::landing::components::web_button::WebButton;

#[component]
pub(crate) fn CtaSection() -> Element {
    rsx! {
        section { class: "mx-auto max-w-6xl px-5 pb-24 lg:px-8",
            div { class: "relative overflow-visible rounded-[24px] border border-zinc-800 bg-zinc-900/90 p-10 text-center shadow-[0_0_0_1px_rgba(255,255,255,0.02)]",
                div { class: "pointer-events-none absolute left-1/2 top-0 h-72 w-[500px] -translate-x-1/2 -translate-y-1/2 rounded-full bg-accent/7 blur-3xl" }
                div { class: "relative",
                    div { class: "mb-3 inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-950 px-3 py-1 text-[10px] uppercase tracking-[0.24em] text-zinc-500",
                        span { class: "h-1.5 w-1.5 rounded-full bg-accent/70" }
                        "Следи за разработкой"
                    }
                    h2 { class: "text-2xl font-semibold tracking-[-0.04em] text-zinc-50 sm:text-3xl", "Разработка в прямом эфире." }
                    p { class: "mx-auto mt-3 max-w-lg text-[14px] leading-relaxed text-zinc-500",
                        "Каждый стрим — реальный коммит. Смотри как строится продукт,"
                        br { class: "hidden sm:inline" }
                        "задавай вопросы, предлагай фичи прямо в чате."
                    }
                    div { class: "mt-7 flex flex-wrap items-center justify-center gap-3",
                        WebButton { large: true }
                        DownloadDropdown { opens_up: true, large: true }
                    }
                    SocialLinks { class_name: "mt-5 flex flex-wrap items-center justify-center gap-4 text-[13px] text-zinc-500", hover_class: "transition hover:text-zinc-200" }
                }
            }
        }
    }
}

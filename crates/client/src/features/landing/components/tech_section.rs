//! Landing page technology section component.

use dioxus::prelude::*;

use crate::features::landing::components::github_icon::GithubIcon;
use crate::features::landing::components::section_intro::SectionIntro;
use crate::features::landing::components::tech_card::TechCard;
use crate::features::landing::data::TECH_GROUPS;

#[component]
pub(crate) fn TechSection() -> Element {
    rsx! {
        section { id: "tech", class: "mx-auto max-w-6xl px-5 pb-20 lg:px-8",
            SectionIntro {
                eyebrow: "Стек",
                title: "Технический стек.",
                description: "Без enterprise-раздутости и legacy-подходов. Весь код — на GitHub."
            }
            div { class: "grid gap-3 sm:grid-cols-2 lg:grid-cols-4",
                for group in TECH_GROUPS {
                    TechCard { group: *group }
                }
            }
            div { class: "mt-4 rounded-[20px] border border-zinc-800 bg-zinc-950/80 p-5",
                div { class: "flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between",
                    div {
                        div { class: "font-mono text-[11px] text-zinc-500", "$ git clone https://github.com/loject/cheenhub" }
                        div { class: "mt-1.5 text-[13px] text-zinc-400", "Весь исходный код открыт. Issues, PR и обсуждения — добро пожаловать." }
                    }
                    a {
                        href: "https://github.com/loject/cheenhub",
                        target: "_blank",
                        rel: "noopener",
                        class: "btn-g flex shrink-0 items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2.5 text-[13px] font-medium text-zinc-200",
                        GithubIcon { class_name: "h-4 w-4" }
                        "Открыть репозиторий"
                    }
                }
            }
        }
    }
}

//! Landing page header component.

use dioxus::prelude::*;

use crate::Route;
use crate::features::landing::components::logo_icon::LogoIcon;

#[component]
pub(crate) fn Header() -> Element {
    rsx! {
        nav { class: "sticky top-0 z-50 border-b border-zinc-800/80 bg-zinc-950/85 backdrop-blur-xl",
            div { class: "mx-auto flex max-w-6xl items-center justify-between px-5 py-3 lg:px-8",
                div { class: "flex items-center gap-3",
                    div { class: "flex h-8 w-8 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-100 text-zinc-950",
                        LogoIcon { class_name: "h-5 w-5" }
                    }
                    span { class: "text-sm font-semibold tracking-[-0.03em] text-zinc-50", "CheenHub" }
                    span { class: "hidden rounded-full border border-zinc-800 bg-zinc-900/80 px-2 py-0.5 text-[10px] uppercase tracking-[0.2em] text-zinc-500 sm:inline-flex", "Бета" }
                }
                div { class: "hidden items-center gap-0.5 sm:flex",
                    a { href: "#features", class: "rounded-lg px-3 py-1.5 text-[13px] text-zinc-400 transition hover:text-zinc-100", "Возможности" }
                    a { href: "#why", class: "rounded-lg px-3 py-1.5 text-[13px] text-zinc-400 transition hover:text-zinc-100", "Сравнение" }
                    a { href: "#tech", class: "rounded-lg px-3 py-1.5 text-[13px] text-zinc-400 transition hover:text-zinc-100", "Стек" }
                }
                div { class: "flex items-center gap-2",
                    a {
                        href: "https://github.com/loject/cheenhub",
                        target: "_blank",
                        rel: "noopener",
                        class: "btn-g hidden items-center rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-[12px] font-medium text-zinc-300 sm:flex",
                        "GitHub"
                    }
                    a {
                        href: "https://www.youtube.com/@cheengeez",
                        target: "_blank",
                        rel: "noopener",
                        class: "btn-g hidden items-center rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-[12px] font-medium text-zinc-300 sm:flex",
                        "YouTube"
                    }
                    a {
                        href: "https://rutube.ru/channel/79753199",
                        target: "_blank",
                        rel: "noopener",
                        class: "btn-g hidden items-center rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-[12px] font-medium text-zinc-300 md:flex",
                        "RuTube"
                    }
                    a {
                        href: "https://t.me/cheenhub_official",
                        target: "_blank",
                        rel: "noopener",
                        class: "btn-g hidden items-center rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-[12px] font-medium text-zinc-300 md:flex",
                        "Telegram"
                    }
                    Link {
                        to: Route::Login { password_reset: None },
                        class: "btn-p flex items-center rounded-xl bg-accent px-3 py-1.5 text-[12px] font-semibold text-white",
                        "Войти"
                    }
                }
            }
        }
    }
}

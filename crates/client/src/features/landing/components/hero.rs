//! Landing page hero component.

use dioxus::prelude::*;

use crate::features::landing::components::download_dropdown::DownloadDropdown;
use crate::features::landing::components::pill::Pill;
use crate::features::landing::components::social_links::SocialLinks;
use crate::features::landing::components::web_button::WebButton;

#[component]
pub(crate) fn Hero() -> Element {
    let app_version = env!("CHEENHUB_APP_VERSION");

    rsx! {
        section { class: "relative mx-auto max-w-6xl px-5 pb-16 pt-20 lg:px-8 lg:pt-28",
            div { class: "pointer-events-none absolute left-1/2 top-0 h-[500px] w-[800px] -translate-x-1/2 rounded-full bg-accent/5 blur-3xl" }
            div { class: "relative",
                div { class: "a1 mb-5 flex justify-center",
                    div { class: "inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/80 px-3.5 py-1.5 text-[11px] uppercase tracking-[0.22em] text-zinc-400",
                        span { class: "relative flex h-2 w-2",
                            span { class: "glow-ring" }
                            span { class: "relative h-2 w-2 rounded-full bg-accent/80" }
                        }
                        "Открытый код  ·  {app_version}  ·  бесплатно"
                    }
                }
                h1 { class: "a2 mx-auto max-w-3xl text-center text-4xl font-semibold leading-[1.1] tracking-[-0.05em] text-zinc-50 sm:text-5xl lg:text-[60px]",
                    "Мгновенная голосовая"
                    br {}
                    span { class: "text-zinc-400", "связь для игр." }
                }
                p { class: "a3 mx-auto mt-5 max-w-xl text-center text-[15px] leading-relaxed text-zinc-500",
                    "Заходи в голосовой канал, зови друзей и общайся без VPN."
                    br { class: "hidden sm:inline" }
                    "Свой сервер — для локального голосового чата в LAN."
                }
                div { class: "a4 relative z-[30] mt-8 flex flex-wrap items-center justify-center gap-3",
                    WebButton { large: false }
                    DownloadDropdown { opens_up: false, large: false, compact: false, opens_right: false }
                }
                SocialLinks { class_name: "a5 mt-5 flex flex-wrap items-center justify-center gap-4 text-[13px] text-zinc-500", hover_class: "transition hover:text-zinc-200" }
                div { class: "a5 relative z-[10] mt-8 flex flex-wrap items-center justify-center gap-2",
                    Pill { strong: "< 1 мс", text: "в LAN на своём сервере" }
                    Pill { strong: "Opus", text: "кодек" }
                    Pill { strong: "Быстрый", text: "WebTransport" }
                    Pill { strong: "MIT", text: "лицензия" }
                    div { class: "flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/80 px-3.5 py-1.5 text-[11px] text-zinc-400", "Нет трекинга" }
                }
            }
        }
    }
}

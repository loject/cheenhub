//! Desktop hero block for the authentication page.

use dioxus::prelude::*;

#[component]
pub(crate) fn AuthHero() -> Element {
    rsx! {
        div { class: "hidden lg:block",
            div { class: "mb-4 inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/80 px-3.5 py-1.5 text-[11px] uppercase tracking-[0.22em] text-zinc-500",
                span { class: "h-2 w-2 rounded-full bg-accent/80" }
                "Аккаунт CheenHub"
            }
            h1 { class: "max-w-xl text-5xl font-semibold leading-[1.05] tracking-[-0.05em] text-zinc-50",
                "Вернись в свои комнаты быстрее."
            }
            p { class: "mt-5 max-w-md text-[15px] leading-relaxed text-zinc-500",
                "Войди в аккаунт, чтобы быстрее возвращаться к своим комнатам, друзьям и настройкам."
            }
        }
    }
}

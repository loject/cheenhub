//! Empty server list panel.

use dioxus::prelude::*;

/// Renders a friendly first-server prompt.
#[component]
pub(crate) fn EmptyServersPanel(on_create_server: EventHandler<()>) -> Element {
    rsx! {
        section { class: "flex min-w-0 flex-1 items-center justify-center bg-zinc-950/35 px-6 py-8",
            div { class: "w-full max-w-[520px] text-center",
                div { class: "mx-auto mb-5 flex h-14 w-14 items-center justify-center rounded-2xl border border-accent/25 bg-accent/10 text-accent",
                    svg { class: "h-6 w-6", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 5v14m-7-7h14" }
                    }
                }
                h1 { class: "text-2xl font-semibold text-zinc-50", "Создай свой первый сервер" }
                p { class: "mx-auto mt-3 max-w-[420px] text-[14px] leading-6 text-zinc-400",
                    "Здесь будут твои серверы, комнаты и голосовые встречи. Начни с небольшого пространства для друзей или команды."
                }
                button {
                    r#type: "button",
                    class: "mt-6 inline-flex h-11 items-center justify-center rounded-xl bg-accent px-5 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)] transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-blue-400",
                    onclick: move |_| on_create_server.call(()),
                    "Создать сервер"
                }
            }
        }
    }
}

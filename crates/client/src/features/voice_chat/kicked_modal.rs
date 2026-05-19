//! Modal shown when the current user is kicked from a voice room.

use dioxus::prelude::*;

/// Informational modal displayed when the server removed the user from a voice room.
#[component]
pub(crate) fn KickedFromVoiceModal(room_name: String, on_close: EventHandler<()>) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-[200] flex items-center justify-center bg-black/65 px-4 py-6 backdrop-blur-sm",
            onclick: move |_| on_close.call(()),

            section {
                role: "dialog",
                "aria-modal": "true",
                "aria-label": "Вы были кикнуты",
                class: "relative w-full max-w-[380px] rounded-2xl border border-zinc-800 bg-zinc-950 p-5 shadow-[0_28px_90px_rgba(0,0,0,0.55)]",
                onclick: move |event| event.stop_propagation(),

                // Icon
                div { class: "mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-xl border border-red-500/25 bg-red-500/10 text-red-400",
                    svg { class: "h-6 w-6", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round",
                            d: "M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9"
                        }
                    }
                }

                h2 { class: "text-center text-[16px] font-semibold text-zinc-50", "Вы были кикнуты" }
                p { class: "mt-1.5 text-center text-[13px] leading-5 text-zinc-400",
                    "Администратор отключил вас от голосовой комнаты "
                    span { class: "font-medium text-zinc-200", "«{room_name}»" }
                    "."
                }

                button {
                    r#type: "button",
                    autofocus: true,
                    class: "mt-5 flex h-10 w-full items-center justify-center rounded-xl bg-zinc-800 text-[13px] font-semibold text-zinc-100 transition-[background,color] duration-100 hover:bg-zinc-700",
                    onclick: move |_| on_close.call(()),
                    "Понятно"
                }
            }
        }
    }
}

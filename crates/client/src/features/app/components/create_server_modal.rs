//! Create-server modal.

use cheenhub_contracts::rest::ServerSummary;
use dioxus::prelude::*;

use crate::features::app::api;

use super::modal::Modal;

/// Renders the server creation flow.
#[component]
pub(crate) fn CreateServerModal(
    on_close: EventHandler<()>,
    on_created: EventHandler<ServerSummary>,
) -> Element {
    let mut name = use_signal(String::new);
    let mut status = use_signal(String::new);
    let mut is_busy = use_signal(|| false);

    rsx! {
        Modal {
            title: "Создать сервер",
            on_close,
            form { class: "space-y-4",
                label { class: "block",
                    span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Название" }
                    input {
                        r#type: "text",
                        name: "server-name",
                        placeholder: "Например, CheenHub Dev",
                        value: name(),
                        maxlength: "48",
                        autocomplete: "off",
                        oninput: move |event| name.set(event.value()),
                        class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[14px] text-zinc-100 outline-none transition placeholder:text-zinc-700 focus:border-accent/70 focus:ring-4 focus:ring-accent/10"
                    }
                }

                if !status().is_empty() {
                    p { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
                        "{status()}"
                    }
                }

                div { class: "flex justify-end gap-2 pt-1",
                    button {
                        r#type: "button",
                        disabled: is_busy(),
                        class: "flex h-10 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 px-4 text-[13px] font-medium text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100 disabled:cursor-not-allowed disabled:opacity-60",
                        onclick: move |_| on_close.call(()),
                        "Отмена"
                    }
                    button {
                        r#type: "button",
                        disabled: is_busy(),
                        class: "flex h-10 items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)] transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-blue-400 disabled:cursor-not-allowed disabled:opacity-60",
                        onclick: move |_| {
                            if is_busy() {
                                return;
                            }
                            is_busy.set(true);
                            status.set(String::new());
                            let request_name = name();
                            spawn(async move {
                                match api::create_server(request_name).await {
                                    Ok(server) => {
                                        on_created.call(server);
                                        on_close.call(());
                                    }
                                    Err(error) => {
                                        status.set(error);
                                        is_busy.set(false);
                                    }
                                }
                            });
                        },
                        if is_busy() { "Создаем..." } else { "Создать" }
                    }
                }
            }
        }
    }
}

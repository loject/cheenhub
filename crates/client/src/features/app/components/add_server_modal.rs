//! Add-server choice modal.

use cheenhub_contracts::rest::ServerSummary;
use dioxus::prelude::*;

use crate::features::app::api;

use super::modal::Modal;

/// Renders the first step for adding a server.
#[component]
pub(crate) fn AddServerModal(
    on_close: EventHandler<()>,
    on_create_server: EventHandler<()>,
    on_joined_server: EventHandler<ServerSummary>,
) -> Element {
    let mut invite = use_signal(String::new);
    let mut status = use_signal(String::new);
    let mut is_joining = use_signal(|| false);

    rsx! {
        Modal {
            title: "Добавить сервер",
            on_close,
            div { class: "space-y-4",
                button {
                    r#type: "button",
                    class: "group flex w-full items-start gap-3 rounded-2xl border border-accent/25 bg-accent/10 p-4 text-left transition-[background,border-color,transform] duration-150 hover:-translate-y-px hover:border-accent/45 hover:bg-accent/15",
                    onclick: move |_| on_create_server.call(()),
                    span { class: "flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-accent text-white shadow-[0_8px_28px_rgba(59,130,246,0.18)]",
                        svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 5v14m-7-7h14" }
                        }
                    }
                    span { class: "min-w-0",
                        span { class: "block text-[14px] font-semibold text-zinc-50", "Создать новый сервер" }
                        span { class: "mt-1 block text-[12px] leading-5 text-zinc-400", "Запусти отдельное пространство для друзей, команды или проекта." }
                    }
                }

                div { class: "rounded-2xl border border-zinc-800 bg-zinc-900/60 p-4",
                    div { class: "flex items-start gap-3",
                        span { class: "flex h-11 w-11 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-zinc-300",
                            svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9" }
                            }
                        }
                        div { class: "min-w-0 flex-1",
                            p { class: "text-[14px] font-semibold text-zinc-50", "Подключиться к серверу" }
                            p { class: "mt-1 text-[12px] leading-5 text-zinc-400", "Вставь ссылку-приглашение или код сервера." }
                        }
                    }
                    div { class: "mt-4 space-y-3",
                        input {
                            r#type: "text",
                            name: "server-invite",
                            placeholder: "cheenhub.ru/invite/team",
                            value: invite(),
                            autocomplete: "off",
                            oninput: move |event| {
                                invite.set(event.value());
                                status.set(String::new());
                            },
                            class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[14px] text-zinc-100 outline-none transition placeholder:text-zinc-700 focus:border-accent/70 focus:ring-4 focus:ring-accent/10"
                        }
                        if !status().is_empty() {
                            p { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
                                "{status()}"
                            }
                        }
                        button {
                            r#type: "button",
                            disabled: is_joining(),
                            class: "flex h-10 w-full items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 px-4 text-[13px] font-medium text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100 disabled:cursor-wait disabled:opacity-70",
                            onclick: move |_| {
                                let Some(code) = invite_code(&invite()) else {
                                    status.set("Вставь ссылку-приглашение или код сервера.".to_owned());
                                    return;
                                };

                                is_joining.set(true);
                                status.set(String::new());
                                spawn(async move {
                                    match api::accept_server_invite(code).await {
                                        Ok(response) => {
                                            on_joined_server.call(response.server);
                                        }
                                        Err(error) => {
                                            status.set(error);
                                            is_joining.set(false);
                                        }
                                    }
                                });
                            },
                            if is_joining() {
                                "Подключаем..."
                            } else {
                                "Подключиться"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn invite_code(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }

    let candidate = trimmed
        .rsplit_once("/invite/")
        .map(|(_, code)| code)
        .unwrap_or(trimmed)
        .trim()
        .trim_matches('/');
    let candidate = candidate
        .find(['?', '#'])
        .map(|index| &candidate[..index])
        .unwrap_or(candidate);

    (!candidate.is_empty()).then(|| candidate.to_owned())
}

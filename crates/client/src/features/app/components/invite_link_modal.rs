//! Invite-link settings modal.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::features::app::api;

use super::modal::Modal;

/// Renders invite-link configuration controls.
#[component]
pub(crate) fn InviteLinkModal(
    server_id: String,
    server_name: String,
    on_close: EventHandler<()>,
) -> Element {
    let mut has_usage_limit = use_signal(|| false);
    let mut usage_limit = use_signal(|| "30".to_owned());
    let mut has_expiration = use_signal(|| false);
    let mut expiration_days = use_signal(|| "30".to_owned());
    let mut generated_link = use_signal(|| None::<String>);
    let mut status = use_signal(String::new);
    let mut is_busy = use_signal(|| false);
    let mut is_copied = use_signal(|| false);
    let mut copy_generation = use_signal(|| 0_u64);
    let limit_panel_class = if has_usage_limit() {
        "max-h-24 translate-y-0 opacity-100"
    } else {
        "pointer-events-none max-h-0 -translate-y-1 opacity-0"
    };
    let expiration_panel_class = if has_expiration() {
        "max-h-24 translate-y-0 opacity-100"
    } else {
        "pointer-events-none max-h-0 -translate-y-1 opacity-0"
    };
    let copy_icon_class = if is_copied() {
        "opacity-0"
    } else {
        "opacity-100"
    };
    let copy_icon_style = if is_copied() {
        "transform: scale(0.72) rotate(-12deg);"
    } else {
        "transform: scale(1) rotate(0deg);"
    };
    let check_icon_class = if is_copied() {
        "opacity-100"
    } else {
        "opacity-0"
    };
    let check_icon_style = if is_copied() {
        "transform: scale(1) rotate(0deg);"
    } else {
        "transform: scale(0.72) rotate(12deg);"
    };

    rsx! {
        Modal {
            title: "Ссылка приглашения",
            on_close,
            div { class: "space-y-4",
                div { class: "rounded-2xl border border-zinc-800 bg-zinc-900/60 p-4",
                    div { class: "flex items-start gap-3",
                        span { class: "flex h-11 w-11 shrink-0 items-center justify-center rounded-xl border border-accent/25 bg-accent/10 text-blue-200",
                            svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M13.19 8.688a4.5 4.5 0 0 1 1.242 7.244l-4.5 4.5a4.5 4.5 0 0 1-6.364-6.364l1.757-1.757m13.35-.622 1.757-1.757a4.5 4.5 0 0 0-6.364-6.364l-4.5 4.5a4.5 4.5 0 0 0 1.242 7.244" }
                            }
                        }
                        div { class: "min-w-0 flex-1",
                            p { class: "truncate text-[14px] font-semibold text-zinc-50", "{server_name}" }
                            p { class: "mt-1 text-[12px] leading-5 text-zinc-400", "При необходимости ограничь срок и количество использований приглашения." }
                        }
                    }
                }

                div { class: "rounded-2xl bg-zinc-900/60 p-3",
                    label { class: "flex min-h-11 cursor-pointer items-center gap-3",
                        input {
                            r#type: "checkbox",
                            checked: has_usage_limit(),
                            onchange: move |event| has_usage_limit.set(event.checked()),
                            class: "h-4 w-4 rounded bg-zinc-950 accent-blue-500"
                        }
                        span { class: "min-w-0",
                            span { class: "block text-[13px] font-medium text-zinc-100", "Задать лимит использований" }
                            span { class: "mt-0.5 block text-[12px] leading-5 text-zinc-500", "Без лимита ссылка будет доступна для любого количества входов." }
                        }
                    }

                    div { class: "overflow-hidden transition-[max-height,opacity,transform] duration-200 ease-out {limit_panel_class}",
                        label { class: "block pt-3",
                            span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Лимит использований" }
                            div { class: "relative",
                                input {
                                    r#type: "number",
                                    min: "1",
                                    max: "999",
                                    step: "1",
                                    inputmode: "numeric",
                                    name: "invite-usage-limit",
                                    autocomplete: "off",
                                    value: "{usage_limit()}",
                                    oninput: move |event| usage_limit.set(event.value()),
                                    class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 pr-32 text-[14px] text-zinc-100 outline-none transition placeholder:text-zinc-700 focus:border-accent/70 focus:ring-4 focus:ring-accent/10"
                                }
                                span { class: "pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-[12px] text-zinc-500", "использований" }
                            }
                        }
                    }
                }

                div { class: "rounded-2xl bg-zinc-900/60 p-3",
                    label { class: "flex min-h-11 cursor-pointer items-center gap-3",
                        input {
                            r#type: "checkbox",
                            checked: has_expiration(),
                            onchange: move |event| has_expiration.set(event.checked()),
                            class: "h-4 w-4 rounded bg-zinc-950 accent-blue-500"
                        }
                        span { class: "min-w-0",
                            span { class: "block text-[13px] font-medium text-zinc-100", "Задать срок действия" }
                            span { class: "mt-0.5 block text-[12px] leading-5 text-zinc-500", "Без срока ссылка останется активной, пока ее не отключат." }
                        }
                    }

                    div { class: "overflow-hidden transition-[max-height,opacity,transform] duration-200 ease-out {expiration_panel_class}",
                        label { class: "block pt-3",
                            span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Срок действия" }
                            div { class: "relative",
                                input {
                                    r#type: "number",
                                    min: "1",
                                    max: "365",
                                    step: "1",
                                    inputmode: "numeric",
                                    name: "invite-expiration-days",
                                    autocomplete: "off",
                                    value: "{expiration_days()}",
                                    oninput: move |event| expiration_days.set(event.value()),
                                    class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 pr-20 text-[14px] text-zinc-100 outline-none transition placeholder:text-zinc-700 focus:border-accent/70 focus:ring-4 focus:ring-accent/10"
                                }
                                span { class: "pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-[12px] text-zinc-500", "дней" }
                            }
                        }
                    }
                }

                if let Some(link) = generated_link() {
                    div { class: "space-y-2 rounded-2xl border border-emerald-500/20 bg-emerald-500/10 p-3",
                        span { class: "block text-[12px] font-medium text-emerald-100", "Готовая ссылка" }
                        div { class: "flex gap-2",
                            input {
                                r#type: "text",
                                readonly: true,
                                value: "{link}",
                                class: "h-11 min-w-0 flex-1 rounded-xl border border-emerald-500/20 bg-zinc-950 px-3 text-[13px] text-zinc-100 outline-none"
                            }
                            button {
                                r#type: "button",
                                class: "relative flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-emerald-500 text-emerald-950 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-emerald-400",
                                "aria-label": if is_copied() { "Ссылка скопирована" } else { "Скопировать ссылку" },
                                onclick: move |_| {
                                    let link_to_copy = link.clone();
                                    is_copied.set(false);
                                    status.set(String::new());
                                    match clipboard_copy(link_to_copy) {
                                        Ok(copy) => {
                                            spawn(async move {
                                                match copy.await {
                                                    Ok(()) => {
                                                        let next_generation = copy_generation() + 1;
                                                        copy_generation.set(next_generation);
                                                        is_copied.set(true);
                                                        TimeoutFuture::new(1400).await;

                                                        if copy_generation() == next_generation {
                                                            is_copied.set(false);
                                                        }
                                                    }
                                                    Err(error) => status.set(error),
                                                }
                                            });
                                        }
                                        Err(error) => status.set(error),
                                    }
                                },
                                span { class: "absolute inset-0 flex items-center justify-center transition-[opacity,transform] duration-200 ease-out {copy_icon_class}", style: copy_icon_style, "aria-hidden": "true",
                                    svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                                        rect { x: "8", y: "8", width: "11", height: "11", rx: "2", ry: "2" }
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" }
                                    }
                                }
                                span { class: "absolute inset-0 flex items-center justify-center transition-[opacity,transform] duration-200 ease-out {check_icon_class}", style: check_icon_style, "aria-hidden": "true",
                                    svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "2.2", view_box: "0 0 24 24",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M20 6 9 17l-5-5" }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "min-h-[38px]",
                    if !status().is_empty() {
                        p { class: "rounded-xl border border-zinc-800 bg-zinc-900/80 px-3 py-2 text-[12px] leading-5 text-zinc-300",
                        "{status()}"
                        }
                    }
                }

                div { class: "flex justify-end gap-2 pt-1",
                    button {
                        r#type: "button",
                        disabled: is_busy(),
                        class: "flex h-10 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 px-4 text-[13px] font-medium text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100",
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

                            let max_uses = match optional_number(
                                has_usage_limit(),
                                usage_limit(),
                                "лимит использований",
                            ) {
                                Ok(value) => value,
                                Err(error) => {
                                    status.set(error);
                                    return;
                                }
                            };
                            let expires_in_days = match optional_number(
                                has_expiration(),
                                expiration_days(),
                                "срок действия",
                            ) {
                                Ok(value) => value,
                                Err(error) => {
                                    status.set(error);
                                    return;
                                }
                            };
                            let request_server_id = server_id.clone();
                            is_busy.set(true);
                            status.set(String::new());
                            generated_link.set(None);

                            spawn(async move {
                                match api::create_server_invite(
                                    request_server_id,
                                    max_uses,
                                    expires_in_days,
                                )
                                .await
                                {
                                    Ok(code) => match current_invite_url(code).await {
                                        Ok(link) => {
                                            generated_link.set(Some(link));
                                            // TODO: show invite creation success in a toast when toasts are available.
                                        }
                                        Err(error) => status.set(error),
                                    },
                                    Err(error) => status.set(error),
                                }
                                is_busy.set(false);
                            });
                        },
                        if is_busy() { "Создаем..." } else { "Создать" }
                    }
                }
            }
        }
    }
}

fn optional_number(enabled: bool, value: String, label: &str) -> Result<Option<u32>, String> {
    if !enabled {
        return Ok(None);
    }

    value
        .trim()
        .parse::<u32>()
        .map(Some)
        .map_err(|_| format!("Проверь {label}."))
}

async fn current_invite_url(code: String) -> Result<String, String> {
    let origin = document::eval("return window.location.origin;")
        .join::<String>()
        .await
        .map_err(|_| "Не удалось определить адрес приложения.".to_owned())?;
    let compact_code = code.replace('-', "");

    Ok(format!(
        "{}/invite/{compact_code}",
        origin.trim_end_matches('/')
    ))
}

fn clipboard_copy(
    link: String,
) -> Result<impl std::future::Future<Output = Result<(), String>>, String> {
    let eval = document::eval(
        r#"
        const link = await dioxus.recv();
        await navigator.clipboard.writeText(link);
        return true;
        "#,
    );
    eval.send(link)
        .map_err(|_| "Не удалось подготовить копирование.".to_owned())?;

    Ok(async move {
        eval.join::<bool>()
            .await
            .map(|_| ())
            .map_err(|_| "Браузер не разрешил скопировать ссылку.".to_owned())
    })
}

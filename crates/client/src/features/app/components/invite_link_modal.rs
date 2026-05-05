//! Invite-link settings modal.

use dioxus::prelude::*;

use super::modal::Modal;

/// Renders invite-link configuration controls.
#[component]
pub(crate) fn InviteLinkModal(server_name: String, on_close: EventHandler<()>) -> Element {
    let mut has_usage_limit = use_signal(|| false);
    let mut usage_limit = use_signal(|| "30".to_owned());
    let mut has_expiration = use_signal(|| false);
    let mut expiration_days = use_signal(|| "30".to_owned());
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

                div { class: "flex justify-end gap-2 pt-1",
                    button {
                        r#type: "button",
                        class: "flex h-10 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 px-4 text-[13px] font-medium text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100",
                        onclick: move |_| on_close.call(()),
                        "Отмена"
                    }
                    button {
                        r#type: "button",
                        class: "flex h-10 items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)] transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-blue-400",
                        onclick: move |_| on_close.call(()),
                        "Создать"
                    }
                }
            }
        }
    }
}

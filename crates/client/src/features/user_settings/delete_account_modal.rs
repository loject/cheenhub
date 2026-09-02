//! Модальное окно подтверждения удаления аккаунта.

use dioxus::prelude::*;

/// Отображает подтверждение начала удаления аккаунта.
#[component]
pub(crate) fn DeleteAccountModal(
    on_close: EventHandler<()>,
    on_confirm: EventHandler<()>,
) -> Element {
    let mut confirmed = use_signal(|| false);

    rsx! {
        div {
            class: "fixed inset-0 z-[200] flex items-center justify-center bg-black/70 px-4 py-6 backdrop-blur-sm",

            button {
                r#type: "button",
                class: "absolute inset-0 cursor-default",
                "aria-label": "Закрыть окно удаления аккаунта",
                onclick: move |_| on_close.call(()),
            }

            section {
                role: "dialog",
                "aria-modal": "true",
                "aria-label": "Подтверждение удаления аккаунта",
                class: "relative max-h-[calc(100dvh-2rem)] w-full max-w-[500px] overflow-y-auto rounded-2xl border border-zinc-800 bg-zinc-950 p-4 text-zinc-100 shadow-[0_30px_110px_rgba(0,0,0,.7)] sm:p-5",

                div { class: "flex items-start justify-between gap-4",
                    div { class: "flex min-w-0 gap-3",
                        div {
                            class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-red-500/20 bg-red-500/10 text-red-300",

                            svg {
                                class: "h-5 w-5",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "1.8",
                                view_box: "0 0 24 24",
                                "aria-hidden": "true",

                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    d: "M3 6h18m-2 0-.8 13a2 2 0 0 1-2 1.9H7.8a2 2 0 0 1-2-1.9L5 6m3 0V4.5A1.5 1.5 0 0 1 9.5 3h5A1.5 1.5 0 0 1 16 4.5V6m-6 4v6m4-6v6"
                                }
                            }
                        }

                        div { class: "min-w-0",
                            h2 {
                                class: "text-[17px] font-semibold tracking-[-0.03em] text-zinc-50",
                                "Удалить аккаунт?"
                            }
                            p {
                                class: "mt-1 text-[12px] leading-5 text-zinc-400",
                                "Это действие отключит ваш аккаунт. В течение 30 дней его можно будет восстановить."
                            }
                        }
                    }

                    button {
                        r#type: "button",
                        class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-400 transition hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50",
                        "aria-label": "Закрыть окно",
                        onclick: move |_| on_close.call(()),

                        svg {
                            class: "h-4 w-4",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            "aria-hidden": "true",

                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                d: "M6 18 18 6M6 6l12 12"
                            }
                        }
                    }
                }

                div {
                    class: "mt-5 rounded-xl border border-red-500/20 bg-red-500/[0.06] p-3.5",

                    div { class: "flex gap-3",
                        svg {
                            class: "mt-0.5 h-4 w-4 shrink-0 text-red-300",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "1.8",
                            view_box: "0 0 24 24",
                            "aria-hidden": "true",

                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                d: "M12 8v5m0 3h.01M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z"
                            }
                        }

                        div {
                            p {
                                class: "text-[12px] font-semibold text-red-100",
                                "Период восстановления — 30 дней"
                            }
                            p {
                                class: "mt-1 text-[11px] leading-5 text-red-100/65",
                                "До окончания этого срока аккаунт можно будет восстановить. После 30 дней восстановление станет невозможно."
                            }
                        }
                    }
                }

                label {
                    class: "mt-4 flex cursor-pointer items-start gap-3 rounded-xl border border-zinc-800 bg-zinc-900/50 p-3.5 transition hover:border-zinc-700 hover:bg-zinc-900/70",

                    input {
                        r#type: "checkbox",
                        checked: confirmed(),
                        onchange: move |event| confirmed.set(event.checked()),
                        class: "mt-0.5 h-4 w-4 shrink-0 cursor-pointer rounded border-zinc-700 bg-zinc-950 accent-red-500 focus-visible:ring-2 focus-visible:ring-red-500/50",
                    }

                    span {
                        class: "text-[12px] leading-5 text-zinc-300",
                        "Я понимаю, что после 30 дней восстановить аккаунт будет невозможно."
                    }
                }

                div {
                    class: "mt-5 flex flex-col-reverse gap-2 border-t border-zinc-800/80 pt-4 sm:flex-row sm:justify-end",

                    button {
                        r#type: "button",
                        class: "flex h-10 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/70 px-4 text-[12px] font-semibold text-zinc-300 transition hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50 sm:h-9",
                        onclick: move |_| on_close.call(()),
                        "Отмена"
                    }

                    button {
                        r#type: "button",
                        disabled: !confirmed(),
                        class: confirm_button_class(confirmed()),
                        onclick: move |_| {
                            if confirmed() {
                                on_confirm.call(());
                            }
                        },
                        "Удалить аккаунт"
                    }
                }
            }
        }
    }
}

fn confirm_button_class(confirmed: bool) -> &'static str {
    if confirmed {
        "flex h-10 items-center justify-center rounded-xl bg-red-500 px-4 text-[12px] font-semibold text-white transition-[background,transform] duration-150 hover:-translate-y-px hover:bg-red-400 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-500/60 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-950 sm:h-9"
    } else {
        "flex h-10 cursor-not-allowed items-center justify-center rounded-xl bg-red-500/30 px-4 text-[12px] font-semibold text-red-100/45 sm:h-9"
    }
}

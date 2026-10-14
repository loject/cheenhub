//! Переиспользуемая панель закреплений с данными, независимыми от типа диалога.

use dioxus::prelude::*;

/// Данные закреплённого сообщения для универсального представления.
#[derive(Clone, PartialEq)]
pub(super) struct PinPreview {
    /// Идентификатор сообщения для команды открепления.
    pub(super) message_id: String,
    /// Имя автора сообщения.
    pub(super) author: String,
    /// Текст сообщения или подпись вложения.
    pub(super) body: String,
    /// Отформатированное время закрепления.
    pub(super) pinned_at: String,
    /// Исходная временная метка закрепления для элемента time.
    pub(super) pinned_at_iso: String,
}

/// Пользовательское действие в панели закреплений.
#[derive(Clone)]
pub(super) enum PinListAction {
    /// Повторить загрузку актуального списка.
    Retry,
    /// Снять закрепление с сообщения.
    Unpin(String),
    /// Перейти к сообщению в истории переписки.
    Navigate(String),
}

/// Показывает закрепления, состояние загрузки и доступные действия.
#[component]
pub(super) fn PinnedMessagesPanel(
    items: Vec<PinPreview>,
    loading: bool,
    error: Option<String>,
    action_error: Option<String>,
    limit: u32,
    can_unpin: bool,
    busy_message_id: Option<String>,
    navigating_message_id: Option<String>,
    navigation_error: Option<String>,
    on_action: EventHandler<PinListAction>,
) -> Element {
    let mut expanded = use_signal(|| false);
    let count = items.len();
    rsx! {
        div { class: "shrink-0 border-b border-white/[0.06] bg-[#0d1016]",
            button {
                r#type: "button",
                class: "flex min-h-11 w-full items-center gap-2.5 px-4 text-left text-[12px] font-medium text-zinc-200 transition-colors hover:bg-white/[0.04]",
                "aria-expanded": if expanded() { "true" } else { "false" },
                "aria-label": "Показать закреплённые сообщения",
                onclick: move |_| expanded.set(!expanded()),
                svg { class: "h-4 w-4 shrink-0 text-blue-300", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "m16 3 5 5-4 1-3 4-1 3-2 2-5-5 2-2 3-1 4-3 1-4zM9 15l-5 6" }
                }
                span { "Закреплённые сообщения" }
                span { class: "rounded-md bg-white/[0.06] px-1.5 py-0.5 text-[10px] tabular-nums text-zinc-400", "{count}/{limit}" }
                if loading || busy_message_id.is_some() || navigating_message_id.is_some() {
                    span { class: "ml-auto h-3.5 w-3.5 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-300", role: "status", "aria-label": "Обновляем закрепления" }
                } else {
                    svg { class: "ml-auto h-4 w-4 text-zinc-500", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: if expanded() { "m6 15 6-6 6 6" } else { "m6 9 6 6 6-6" } }
                    }
                }
            }
            if let Some(action_error) = action_error {
                div { role: "alert", class: "mx-3 mb-2 rounded-lg bg-red-500/10 px-3 py-2 text-[11px] text-red-200", "{action_error}" }
            }
            if let Some(navigation_error) = navigation_error {
                div { role: "alert", class: "mx-3 mb-2 rounded-lg bg-amber-400/10 px-3 py-2 text-[11px] text-amber-100", "{navigation_error}" }
            }
            if expanded() {
                div { class: "max-h-[min(50vh,22rem)] space-y-2 overflow-y-auto border-t border-white/[0.05] p-3",
                    if loading && items.is_empty() {
                        div { role: "status", class: "py-5 text-center text-[12px] text-zinc-400", "Загружаем закрепления…" }
                    }
                    if let Some(error) = error {
                        div { class: "rounded-xl bg-red-500/[0.08] p-3 text-center text-[12px] text-red-200",
                            p { "{error}" }
                            button { r#type: "button", class: "mt-2 rounded-lg bg-red-500/10 px-3 py-2 text-red-100 hover:bg-red-500/20",
                                onclick: move |_| on_action.call(PinListAction::Retry),
                                "Повторить"
                            }
                        }
                    } else if !loading && items.is_empty() {
                        div { class: "rounded-xl bg-white/[0.03] px-4 py-6 text-center",
                            p { class: "text-[13px] font-medium text-zinc-200", "Пока ничего не закреплено" }
                            p { class: "mt-1 text-[11px] leading-5 text-zinc-500",
                                if can_unpin {
                                    "Открой контекстное меню сообщения, чтобы сохранить его здесь."
                                } else {
                                    "Когда важное сообщение закрепят, оно появится здесь."
                                }
                            }
                        }
                    } else {
                        for item in items.iter().cloned() {
                            div { key: "{item.message_id}", class: "rounded-xl border border-white/[0.06] bg-white/[0.025] p-3",
                                button {
                                    r#type: "button",
                                    disabled: navigating_message_id.is_some(),
                                    class: "w-full rounded-lg px-1 py-1 text-left transition-colors hover:bg-blue-400/[0.07] focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-400 disabled:cursor-wait disabled:opacity-60",
                                    "aria-label": "Перейти к закреплённому сообщению",
                                    onclick: {
                                        let id = item.message_id.clone();
                                        move |_| on_action.call(PinListAction::Navigate(id.clone()))
                                    },
                                    div { class: "flex items-center justify-between gap-3",
                                        span { class: "truncate text-[11px] font-semibold text-zinc-200", "{item.author}" }
                                        time { class: "shrink-0 text-[10px] text-zinc-500", datetime: "{item.pinned_at_iso}", "{item.pinned_at}" }
                                    }
                                    p { class: "mt-1.5 max-h-20 overflow-hidden whitespace-pre-wrap break-words text-[12px] leading-5 text-zinc-400", "{item.body}" }
                                    if navigating_message_id.as_deref() == Some(item.message_id.as_str()) {
                                        span { class: "mt-2 block text-[11px] text-blue-300", role: "status", "Ищем сообщение в истории…" }
                                    }
                                }
                                if can_unpin {
                                    button {
                                        r#type: "button",
                                        disabled: busy_message_id.is_some(),
                                        class: "mt-2 min-h-8 rounded-lg px-2.5 text-[11px] font-medium text-zinc-400 transition-colors hover:bg-red-500/10 hover:text-red-200 disabled:cursor-wait disabled:opacity-40",
                                        onclick: {
                                            let id = item.message_id.clone();
                                            move |_| on_action.call(PinListAction::Unpin(id.clone()))
                                        },
                                        "Открепить"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

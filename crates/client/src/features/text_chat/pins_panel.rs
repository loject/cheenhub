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
    let mut action_menu_id = use_signal(|| None::<String>);
    let count = items.len();
    rsx! {
        div {
            class: "relative z-30 shrink-0 border-b border-white/[0.06] bg-[#0d1016]",
            onkeydown: move |event| {
                if event.key() == Key::Escape {
                    expanded.set(false);
                    action_menu_id.set(None);
                }
            },
            button {
                r#type: "button",
                class: "group flex min-h-[48px] w-full items-center gap-3 px-5 py-2 text-left text-zinc-200 transition-colors hover:bg-white/[0.035] focus-visible:outline focus-visible:outline-2 focus-visible:outline-inset focus-visible:outline-blue-400",
                "aria-expanded": if expanded() { "true" } else { "false" },
                "aria-label": "Показать закреплённые сообщения",
                onclick: move |_| {
                    expanded.set(!expanded());
                    action_menu_id.set(None);
                },
                div { class: "flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-blue-400/[0.08] text-blue-300",
                    svg { class: "h-4 w-4", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "m16 3 5 5-4 1-3 4-1 3-2 2-5-5 2-2 3-1 4-3 1-4zM9 15l-5 6" }
                    }
                }
                div { class: "flex min-w-0 flex-1 items-center gap-3",
                    span { class: "shrink-0 text-[12px] font-semibold text-zinc-200", "Закреплено" }
                    span { class: "h-4 w-px shrink-0 bg-white/[0.1]" }
                    if let Some(item) = items.first() {
                        p { class: "min-w-0 flex-1 truncate text-[12px] leading-5 text-zinc-400",
                            span { class: "font-medium text-zinc-300", "{item.author}" }
                            span { class: "px-1.5 text-zinc-600", "·" }
                            "{item.body}"
                        }
                    } else if loading {
                        p { class: "truncate text-[12px] text-zinc-500", "Загружаем сообщения…" }
                    } else {
                        p { class: "truncate text-[12px] text-zinc-500", "Нет закреплённых сообщений" }
                    }
                }
                if count > 0 {
                    span { class: "shrink-0 rounded-md bg-white/[0.055] px-2 py-0.5 text-[10px] font-medium tabular-nums text-zinc-400", "{count}" }
                }
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
                div {
                    class: "fixed inset-0 z-30",
                    role: "presentation",
                    onclick: move |_| {
                        expanded.set(false);
                        action_menu_id.set(None);
                    },
                }
                div {
                    class: "absolute right-3 top-[calc(100%+8px)] z-40 w-[min(30rem,calc(100vw-1.5rem))] max-h-[min(65vh,30rem)] space-y-1 overflow-y-auto rounded-xl border border-white/[0.1] bg-[#15171d] p-2 shadow-[0_18px_55px_rgba(0,0,0,0.55)]",
                    role: "dialog",
                    "aria-label": "Закреплённые сообщения",
                    div { class: "sticky top-0 z-10 mb-1 flex items-center justify-between gap-3 border-b border-white/[0.065] bg-[#15171d] px-3 pb-3 pt-2",
                        div { class: "flex items-center gap-2",
                            span { class: "text-[13px] font-semibold text-zinc-100", "Закреплённые сообщения" }
                            span { class: "rounded-md bg-white/[0.055] px-1.5 py-0.5 text-[10px] tabular-nums text-zinc-400", "{count} из {limit}" }
                        }
                        button {
                            r#type: "button",
                            class: "flex h-7 w-7 items-center justify-center rounded-lg text-zinc-500 transition-colors hover:bg-white/[0.06] hover:text-zinc-200",
                            "aria-label": "Закрыть закреплённые сообщения",
                            onclick: move |_| {
                                expanded.set(false);
                                action_menu_id.set(None);
                            },
                            svg { class: "h-4 w-4", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M18 6 6 18M6 6l12 12" }
                            }
                        }
                    }
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
                        div { class: "rounded-lg bg-white/[0.025] px-4 py-5 text-center",
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
                            div { key: "{item.message_id}", class: "group relative rounded-lg border border-transparent transition-colors hover:border-white/[0.06] hover:bg-white/[0.04]",
                                div { class: "pointer-events-none absolute bottom-3 left-1 top-3 w-[2px] rounded-full bg-blue-400/50" }
                                div { class: "flex items-center gap-1 pl-3 pr-1",
                                    button {
                                        r#type: "button",
                                        disabled: navigating_message_id.is_some(),
                                        class: "min-w-0 flex-1 rounded-lg px-2 py-3 text-left focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-400 disabled:cursor-wait disabled:opacity-60",
                                        "aria-label": "Перейти к закреплённому сообщению",
                                        onclick: {
                                            let id = item.message_id.clone();
                                            move |_| {
                                                expanded.set(false);
                                                action_menu_id.set(None);
                                                on_action.call(PinListAction::Navigate(id.clone()));
                                            }
                                        },
                                        div { class: "flex min-w-0 items-baseline justify-between gap-3",
                                            span { class: "truncate text-[12px] font-semibold text-zinc-200", "{item.author}" }
                                            time { class: "shrink-0 text-[10px] tabular-nums text-zinc-500", datetime: "{item.pinned_at_iso}", "{item.pinned_at}" }
                                        }
                                        p { class: "mt-1 line-clamp-2 break-words text-[12px] leading-[19px] text-zinc-400 [overflow-wrap:anywhere]", "{item.body}" }
                                        if navigating_message_id.as_deref() == Some(item.message_id.as_str()) {
                                            span { class: "mt-1 block text-[11px] text-blue-300", role: "status", "Ищем сообщение…" }
                                        }
                                    }
                                    if can_unpin {
                                        button {
                                            r#type: "button",
                                            disabled: busy_message_id.is_some(),
                                            class: "flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-zinc-500 transition-colors hover:bg-white/[0.07] hover:text-zinc-200 disabled:cursor-wait disabled:opacity-40",
                                            "aria-label": "Действия с закреплённым сообщением",
                                            "aria-expanded": if action_menu_id().as_deref() == Some(item.message_id.as_str()) { "true" } else { "false" },
                                            onclick: {
                                                let id = item.message_id.clone();
                                                move |_| {
                                                    if action_menu_id().as_deref() == Some(id.as_str()) {
                                                        action_menu_id.set(None);
                                                    } else {
                                                        action_menu_id.set(Some(id.clone()));
                                                    }
                                                }
                                            },
                                            svg { class: "h-4 w-4", view_box: "0 0 24 24", fill: "currentColor",
                                                circle { cx: "5", cy: "12", r: "1.6" }
                                                circle { cx: "12", cy: "12", r: "1.6" }
                                                circle { cx: "19", cy: "12", r: "1.6" }
                                            }
                                        }
                                    }
                                }
                                if can_unpin && action_menu_id().as_deref() == Some(item.message_id.as_str()) {
                                    div { class: "flex justify-end px-2 pb-2",
                                        button {
                                            r#type: "button",
                                            disabled: busy_message_id.is_some(),
                                            class: "rounded-lg px-3 py-2 text-[11px] font-medium text-red-300 transition-colors hover:bg-red-500/10 hover:text-red-200 disabled:cursor-wait disabled:opacity-40",
                                            onclick: {
                                                let id = item.message_id.clone();
                                                move |_| {
                                                    action_menu_id.set(None);
                                                    on_action.call(PinListAction::Unpin(id.clone()));
                                                }
                                            },
                                            "Открепить сообщение"
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
}

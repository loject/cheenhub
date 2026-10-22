//! Встроенное редактирование текста собственного сообщения комнаты.

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;

/// Состояние единственного открытого редактора в панели комнаты.
///
/// Коллбэки сохраняют обработку команд в панели, а не внутри строки сообщения.
#[derive(Clone, PartialEq)]
pub(crate) struct InlineMessageEditState {
    /// Сообщение, которое редактируется в текущий момент.
    pub(crate) message_id: String,
    /// Запрос на сохранение ещё выполняется.
    pub(crate) saving: bool,
    /// Ошибка последней попытки сохранения.
    pub(crate) error: Option<String>,
    /// Закрыть редактор без изменения сообщения.
    pub(crate) on_cancel: EventHandler<()>,
    /// Передать новый текст в панель для сохранения.
    pub(crate) on_save: EventHandler<String>,
}

/// Проверяет, можно ли сохранить изменённый текст без изменения вложений.
pub(super) fn can_save_edit(body: &str, original: &str, has_attachment: bool) -> bool {
    let trimmed = body.trim();
    (has_attachment || !trimmed.is_empty())
        && trimmed.chars().count() <= 2000
        && trimmed != original
}

/// Показывает поле редактирования прямо внутри сообщения, сохраняя вложения.
#[component]
pub(crate) fn RoomMessageInlineEditor(
    message: TextChatMessage,
    saving: bool,
    error: Option<String>,
    on_cancel: EventHandler<()>,
    on_save: EventHandler<String>,
) -> Element {
    let initial = message.body.clone();
    let mut draft = use_signal(|| initial);
    let body = draft();
    let original_for_submit = message.body.clone();
    let original_for_keys = message.body.clone();
    let has_attachment = !message.attachments.is_empty();
    let valid = can_save_edit(&body, &message.body, has_attachment);

    rsx! {
        div {
            class: if saving { "chat-inline-editor chat-inline-editor-saving w-full min-w-0" } else { "chat-inline-editor w-full min-w-0" },
            role: "group",
            "aria-label": "Редактирование сообщения",
            form {
                class: "flex flex-col gap-2.5",
                onsubmit: move |event| {
                    event.prevent_default();
                    let next = draft();
                    if !saving && can_save_edit(&next, &original_for_submit, has_attachment) {
                        on_save.call(next.trim().to_owned());
                    }
                },
                div {
                    class: "flex items-center gap-2 text-[11px] font-semibold tracking-[0.02em] text-blue-200",
                    svg {
                        class: "h-3.5 w-3.5 shrink-0 text-blue-300",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "1.8",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "m16.9 3.9 3.2 3.2M4 20l4.5-1 11-11a2.3 2.3 0 0 0-3.2-3.2l-11 11L4 20z" }
                    }
                    "Редактирование сообщения"
                }
                textarea {
                    autofocus: true,
                    rows: "2",
                    maxlength: "2000",
                    style: "field-sizing: content;",
                    value: "{body}",
                    disabled: saving,
                    placeholder: "Напиши текст сообщения…",
                    "aria-label": "Текст редактируемого сообщения",
                    onmounted: move |event| {
                        spawn(async move {
                            if let Err(error) = event.data().set_focus(true).await {
                                debug!(%error, "could not focus inline room message editor");
                            }
                        });
                    },
                    class: "chat-inline-editor-input max-h-[38dvh] min-h-[54px] w-full min-w-0 resize-y overflow-y-auto rounded-lg border border-white/10 bg-[#090f18]/80 px-3 py-2.5 text-left text-[14px] leading-[22px] text-zinc-100 outline-none transition-[border-color,background-color,box-shadow] duration-200 placeholder:text-zinc-600 focus:border-blue-400/60 focus:bg-[#0d1521] focus:ring-[3px] focus:ring-blue-500/10 disabled:cursor-wait disabled:opacity-60",
                    oninput: move |event| draft.set(event.value()),
                    onkeydown: move |event| {
                        if event.key() == Key::Escape && !saving {
                            event.prevent_default();
                            event.stop_propagation();
                            on_cancel.call(());
                        } else if event.key() == Key::Enter
                            && (event.modifiers().ctrl() || event.modifiers().meta())
                            && !event.is_composing()
                        {
                            event.prevent_default();
                            event.stop_propagation();
                            let next = draft();
                            if !saving && can_save_edit(&next, &original_for_keys, has_attachment) {
                                on_save.call(next.trim().to_owned());
                            }
                        }
                    },
                }
                if let Some(error) = error {
                    p {
                        class: "chat-inline-editor-error rounded-lg border border-red-400/20 bg-red-500/10 px-3 py-2 text-[12px] text-red-200",
                        role: "alert",
                        "{error}"
                    }
                }
                div {
                    class: "chat-inline-editor-actions flex flex-wrap items-center gap-x-3 gap-y-2",
                    span {
                        class: "hidden text-[10.5px] text-zinc-500 sm:inline",
                        "Esc — отмена · Ctrl+Enter — сохранить"
                    }
                    span {
                        class: if body.chars().count() >= 1900 { "ml-auto text-[10.5px] tabular-nums text-amber-300" } else { "ml-auto text-[10.5px] tabular-nums text-zinc-500" },
                        "{body.chars().count()} / 2000"
                    }
                    button {
                        r#type: "button",
                        disabled: saving,
                        class: "h-8 rounded-lg px-3 text-[12px] font-medium text-zinc-300 transition-[background-color,color,transform] duration-150 hover:bg-white/10 hover:text-white active:scale-[0.97] disabled:cursor-not-allowed disabled:opacity-40",
                        onclick: move |_| on_cancel.call(()),
                        "Отмена"
                    }
                    button {
                        r#type: "submit",
                        disabled: saving || !valid,
                        class: "inline-flex h-8 items-center justify-center gap-2 rounded-lg bg-blue-500 px-3.5 text-[12px] font-semibold text-white shadow-[0_3px_14px_rgba(59,130,246,0.20)] transition-[background-color,box-shadow,transform] duration-150 hover:bg-blue-400 hover:shadow-[0_5px_20px_rgba(59,130,246,0.3)] active:scale-[0.97] disabled:cursor-not-allowed disabled:opacity-45 disabled:shadow-none",
                        if saving {
                            span {
                                class: "h-3.5 w-3.5 animate-spin rounded-full border-2 border-white/30 border-t-white",
                                "aria-hidden": "true"
                            }
                            "Сохраняем…"
                        } else {
                            "Сохранить"
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;

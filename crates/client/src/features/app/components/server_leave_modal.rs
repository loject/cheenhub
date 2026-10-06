//! Подтверждение выхода с сервера с состоянием запроса и повторной попыткой.

use super::modal::Modal;
use crate::features::app::api;
use dioxus::prelude::*;

/// Выходит с указанного сервера только после подтверждения пользователя.
///
/// При ошибке диалог остаётся открытым; во время запроса закрытие заблокировано.
#[component]
pub(super) fn ServerLeaveModal(
    server_id: String,
    server_name: String,
    on_close: EventHandler<()>,
    on_left: EventHandler<String>,
) -> Element {
    let mut busy = use_signal(|| false);
    let mut error = use_signal(String::new);
    rsx! {
        Modal {
            title: "Выйти с сервера",
            close_disabled: busy(),
            on_close: move |_| { if !busy() { on_close.call(()); } },
            p { class: "text-[13px] leading-6 text-zinc-400",
                "Выйти с сервера «{server_name}»? Чтобы вернуться, понадобится приглашение."
            }
            if !error().is_empty() {
                p { role: "alert", class: "mt-3 rounded-xl bg-red-500/10 p-3 text-sm text-red-200", "{error()}" }
            }
            div { class: "mt-5 flex justify-end gap-2",
                button {
                    r#type: "button", disabled: busy(),
                    class: "h-10 rounded-xl border border-zinc-800 px-4 text-sm hover:bg-zinc-800 disabled:opacity-50",
                    onclick: move |_| on_close.call(()), "Отмена"
                }
                button {
                    r#type: "button", disabled: busy(),
                    class: "flex h-10 items-center gap-2 rounded-xl bg-red-500 px-4 text-sm font-semibold text-white hover:bg-red-400 disabled:opacity-60",
                    onclick: move |_| {
                        if busy() { return; }
                        busy.set(true);
                        error.set(String::new());
                        let id = server_id.clone();
                        spawn(async move {
                            info!(server_id = %id, "leaving server after confirmation");
                            match api::leave_server(id.clone()).await {
                                Ok(()) => {
                                    info!(server_id = %id, "left server");
                                    on_left.call(id);
                                }
                                Err(message) => {
                                    warn!(server_id = %id, %message, "failed to leave server");
                                    error.set(message);
                                    busy.set(false);
                                }
                            }
                        });
                    },
                    if busy() { span { class: "size-4 animate-spin rounded-full border-2 border-white/30 border-t-white", "aria-hidden": "true" } }
                    if busy() { "Выходим…" } else { "Выйти с сервера" }
                }
            }
        }
    }
}

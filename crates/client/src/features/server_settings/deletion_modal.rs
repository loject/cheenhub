//! Подтверждение и выполнение удаления сервера в границе настроек.

use super::api;
use crate::features::app::components::modal::Modal;
use cheenhub_contracts::rest::ServerSummary;
use dioxus::prelude::*;

/// Удаляет принадлежащий пользователю сервер после явного подтверждения.
///
/// Вызывающий scope размещает диалог вне фильтрованных панелей и удаляет
/// рабочую область только после успешного завершения команды.
#[component]
pub(super) fn ServerDeletionModal(
    server: ServerSummary,
    on_close: EventHandler<()>,
    on_deleted: EventHandler<String>,
) -> Element {
    let mut busy = use_signal(|| false);
    let mut error = use_signal(String::new);
    rsx! {
        Modal {
            title: "Удалить сервер",
            close_disabled: busy(),
            on_close: move |_| { if !busy() { on_close.call(()); } },
            p { class: "text-[13px] leading-6 text-zinc-400",
                "Сервер «{server.name}», его комнаты, сообщения и приглашения будут удалены без возможности восстановления."
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
                    r#type: "button", disabled: busy() || !server.is_owner,
                    class: "flex h-10 items-center gap-2 rounded-xl bg-red-500 px-4 text-sm font-semibold text-white hover:bg-red-400 disabled:opacity-60",
                    onclick: move |_| {
                        if busy() || !server.is_owner { return; }
                        busy.set(true);
                        error.set(String::new());
                        let id = server.id.clone();
                        spawn(async move {
                            info!(server_id = %id, "deleting own server after confirmation");
                            match api::delete_server(id.clone()).await {
                                Ok(()) => {
                                    info!(server_id = %id, "own server deleted");
                                    on_deleted.call(id);
                                }
                                Err(message) => {
                                    warn!(server_id = %id, %message, "failed to delete own server");
                                    error.set(message);
                                    busy.set(false);
                                }
                            }
                        });
                    },
                    if busy() { span { class: "size-4 animate-spin rounded-full border-2 border-white/30 border-t-white", "aria-hidden": "true" } }
                    if busy() { "Удаляем…" } else { "Удалить сервер" }
                }
            }
        }
    }
}

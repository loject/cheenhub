//! Диалог подтверждения удаления комнаты сервера.

use dioxus::prelude::*;

use super::modal::Modal;

/// Спрашивает подтверждение перед необратимым удалением комнаты.
///
/// Удаление уничтожает комнату вместе с её сообщениями, поэтому действие требует
/// явного согласия пользователя. Пока `is_deleting` истинно, кнопки закрытия и
/// отмены заблокированы, чтобы запрос не остался без визуального ответа.
#[component]
pub(super) fn RoomDeleteConfirm(
    room_name: String,
    is_deleting: bool,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    let action_label = if is_deleting {
        "Удаляем..."
    } else {
        "Удалить"
    };

    rsx! {
        Modal {
            title: "Удалить комнату",
            on_close: move |_| {
                if !is_deleting {
                    on_cancel.call(());
                }
            },
            p { class: "text-[13px] leading-6 text-zinc-400",
                "Комната «{room_name}» и её сообщения будут удалены без возможности восстановления."
            }
            div { class: "mt-5 flex justify-end gap-2",
                button {
                    r#type: "button",
                    disabled: is_deleting,
                    class: "flex h-10 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 px-4 text-[13px] font-medium text-zinc-300 transition hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100 disabled:cursor-not-allowed disabled:opacity-60",
                    onclick: move |_| on_cancel.call(()),
                    "Отмена"
                },
                button {
                    r#type: "button",
                    disabled: is_deleting,
                    class: "flex h-10 items-center justify-center rounded-xl bg-red-500 px-4 text-[13px] font-semibold text-white transition hover:bg-red-400 disabled:cursor-not-allowed disabled:opacity-60",
                    onclick: move |_| {
                        if is_deleting {
                            return;
                        }
                        on_confirm.call(());
                    },
                    "{action_label}"
                }
            }
        }
    }
}

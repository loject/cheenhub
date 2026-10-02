//! Ошибка первоначальной загрузки комнат сервера.

use dioxus::prelude::*;

/// Показывает пользователю ошибку загрузки списка комнат и позволяет повторить запрос.
///
/// Комнаты не восстановятся без явного действия пользователя, поэтому блок всегда
/// предоставляет следующий шаг вместо «мёртвого» сообщения об ошибке.
#[component]
pub(super) fn ServerRoomsLoadError(
    message: String,
    is_retrying: bool,
    on_retry: EventHandler<()>,
) -> Element {
    let retry_label = if is_retrying {
        "Повторяем..."
    } else {
        "Повторить"
    };

    rsx! {
        div { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
            "{message}"
            button {
                r#type: "button",
                disabled: is_retrying,
                class: "mt-2 flex h-8 items-center justify-center rounded-lg border border-red-400/30 bg-red-500/10 px-3 text-[12px] font-semibold text-red-100 transition hover:bg-red-500/20 disabled:cursor-not-allowed disabled:opacity-60",
                onclick: move |_| {
                    if is_retrying {
                        return;
                    }
                    on_retry.call(());
                },
                "{retry_label}"
            }
        }
    }
}

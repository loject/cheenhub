//! Ошибка операции над комнатами сервера.

use dioxus::prelude::*;

/// Показывает пользователю ошибку создания, изменения или удаления комнаты.
#[component]
pub(super) fn ServerRoomsActionError(message: String) -> Element {
    rsx! {
        p { class: "mt-3 rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[11px] leading-4 text-red-200",
            "{message}"
        }
    }
}

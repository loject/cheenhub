//! Ошибка первоначальной загрузки комнат сервера.

use dioxus::prelude::*;

/// Показывает пользователю ошибку загрузки списка комнат.
#[component]
pub(super) fn ServerRoomsLoadError(message: String) -> Element {
    rsx! {
        div { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
            "{message}"
        }
    }
}

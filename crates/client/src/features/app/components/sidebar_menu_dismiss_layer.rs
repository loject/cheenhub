//! Полноэкранный слой закрытия боковых меню.

use dioxus::prelude::*;

/// Закрывает открытое меню после нажатия за пределами боковой панели.
#[component]
pub(crate) fn SidebarMenuDismissLayer(on_close: EventHandler<()>) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-[94] cursor-default",
            "aria-label": "Закрыть открытое меню",
            onclick: move |_| on_close.call(()),
        }
    }
}

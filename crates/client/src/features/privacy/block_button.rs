//! Кнопка блокировки собеседника в личном диалоге.

use dioxus::prelude::*;

/// Передаёт выбранного пользователя владельцу страницы для показа подтверждения.
#[component]
pub(crate) fn PrivacyBlockButton(
    user_id: String,
    nickname: String,
    compact: bool,
    on_open: EventHandler<(String, String)>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: if compact {
                "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-red-500/20 bg-red-500/5 text-red-300 transition hover:bg-red-500/15"
            } else {
                "flex w-full items-center gap-2 rounded-xl px-3 py-2.5 text-left text-[13px] text-red-300 hover:bg-red-500/10"
            },
            "aria-label": "Заблокировать {nickname}",
            title: "Заблокировать {nickname}",
            onclick: move |_| {
                debug!(%user_id, "opening direct message block confirmation");
                on_open.call((user_id.clone(), nickname.clone()));
            },
            svg { class: "h-4 w-4 shrink-0", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                circle { cx: "9", cy: "7", r: "3" }
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3 20v-2a6 6 0 0 1 8.5-5.5" }
                circle { cx: "17", cy: "17", r: "5" }
                path { stroke_linecap: "round", d: "m13.5 20.5 7-7" }
            }
            if !compact { "Заблокировать" }
        }
    }
}

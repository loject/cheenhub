//! Пустое и ошибочное состояние списка экранов.

use dioxus::prelude::*;

/// Визуальный тон пустого состояния.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum EmptyStateTone {
    /// Нейтральное состояние без ошибки.
    #[default]
    Neutral,
    /// Состояние ошибки загрузки.
    Danger,
}

/// Показывает объяснение и необязательное действие для пустого списка.
///
/// Пустая строка в `action_label` скрывает кнопку действия.
#[component]
pub(super) fn EmptyState(
    #[props(into)] title: String,
    #[props(into)] body: String,
    #[props(into, default)] action_label: String,
    #[props(default)] tone: EmptyStateTone,
    #[props(default)] on_action: EventHandler<()>,
) -> Element {
    let danger = tone == EmptyStateTone::Danger;
    let panel_class = if danger {
        "screen-share-fade-up flex h-[128px] flex-col items-center justify-center rounded-lg border border-red-400/20 bg-red-500/[.04] px-5 text-center"
    } else {
        "screen-share-fade-up flex h-[128px] flex-col items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900/40 px-5 text-center"
    };
    let icon_class = if danger {
        "grid h-9 w-9 place-items-center rounded-lg bg-red-500/10 text-red-300"
    } else {
        "grid h-9 w-9 place-items-center rounded-lg bg-zinc-800 text-zinc-300"
    };
    let action_class = if danger {
        "mt-3 inline-flex min-h-9 items-center rounded-lg border border-red-400/25 bg-red-500/10 px-3 text-[12px] font-semibold text-red-100 transition-[background-color,border-color] duration-200 hover:border-red-300/40 hover:bg-red-500/15 active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-300/70"
    } else {
        "mt-3 inline-flex min-h-9 items-center rounded-lg border border-zinc-700 bg-zinc-900 px-3 text-[12px] font-semibold text-zinc-100 transition-[background-color,border-color] duration-200 hover:border-zinc-600 hover:bg-zinc-800 active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/70"
    };

    rsx! {
        div {
            class: panel_class,
            role: if danger { "alert" } else { "status" },
            div { class: icon_class,
                if danger {
                    svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 9v4m0 4h.01M10.3 3.9 2.6 17.2A2 2 0 0 0 4.3 20h15.4a2 2 0 0 0 1.7-2.8L13.7 3.9a2 2 0 0 0-3.4 0Z" }
                    }
                } else {
                    svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                        rect { x: "3", y: "4", width: "18", height: "12", rx: "2" }
                        path { stroke_linecap: "round", d: "M8 20h8m-4-4v4" }
                    }
                }
            }
            h3 { class: "mt-2.5 text-[13px] font-semibold text-zinc-100", "{title}" }
            p { class: "mt-1 max-w-sm text-[11.5px] leading-5 text-zinc-400", "{body}" }
            if !action_label.is_empty() {
                button {
                    r#type: "button",
                    class: action_class,
                    onclick: move |_| on_action.call(()),
                    "{action_label}"
                }
            }
        }
    }
}

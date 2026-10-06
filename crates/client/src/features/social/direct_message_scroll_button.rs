//! Кнопка перехода к последнему сообщению личного диалога.

use dioxus::prelude::*;

use crate::features::text_chat::ScrollCommand;

/// Показывает кнопку возврата в конец истории, когда список прокручен вверх.
///
/// Кнопка позиционируется поверх списка сообщений, поэтому отрисовывается в
/// контейнере с `relative`, который владеет областью истории диалога.
#[component]
pub(super) fn DirectMessageScrollButton(
    visible: bool,
    on_scroll_to_bottom: EventHandler<()>,
) -> Element {
    if !visible {
        return rsx! {};
    }

    rsx! {
        div { class: "pointer-events-none absolute bottom-3 right-4 z-20",
            button {
                r#type: "button",
                class: "group pointer-events-auto relative flex h-10 w-10 items-center justify-center rounded-full bg-zinc-900/95 text-blue-200 shadow-[0_8px_22px_rgba(0,0,0,0.35),0_0_0_1px_rgba(255,255,255,0.08)] transition-[background-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-zinc-800 hover:text-blue-100 active:scale-[0.96]",
                "aria-label": "Перейти к последнему сообщению",
                onclick: move |_| on_scroll_to_bottom.call(()),
                span { class: "pointer-events-none absolute bottom-[calc(100%+8px)] right-0 whitespace-nowrap rounded-lg border border-zinc-800 bg-zinc-950/95 px-2 py-1 text-[11px] font-medium text-zinc-300 opacity-0 shadow-[0_8px_22px_rgba(0,0,0,0.35)] transition-[opacity,transform] duration-150 group-hover:opacity-100",
                    "К последнему сообщению"
                }
                svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 5v14m0 0 6-6m-6 6-6-6" }
                }
            }
        }
    }
}

/// Команда прокрутки, используемая кнопкой возврата в конец истории.
pub(super) fn scroll_to_bottom() -> ScrollCommand {
    ScrollCommand::SmoothBottom
}

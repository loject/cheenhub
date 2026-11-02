//! Сегментированный переключатель параметров качества.

use dioxus::prelude::*;

use super::decor::aria_flag;

/// Вариант сегментированного переключателя.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SegmentedOption {
    /// Значение варианта для HTML-атрибутов и событий.
    pub(super) value: String,
    /// Основная подпись варианта.
    pub(super) label: String,
    /// Уточняющая подпись под основным значением.
    pub(super) hint: String,
}

/// Показывает набор взаимоисключающих вариантов и сообщает выбранное значение.
///
/// `columns` задаёт CSS-классы сетки, чтобы одна и та же форма работала и для
/// двух разрешений, и для трёх частот кадров.
#[component]
pub(super) fn Segmented(
    #[props(into)] label: String,
    #[props(into)] columns: String,
    options: Vec<SegmentedOption>,
    #[props(into)] selected: String,
    on_change: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            class: "grid gap-1 rounded-lg bg-zinc-900/60 p-1 {columns}",
            role: "radiogroup",
            "aria-label": "{label}",
            for option in options {
                {option_button(option, selected.clone(), on_change)}
            }
        }
    }
}

fn option_button(
    option: SegmentedOption,
    selected: String,
    on_change: EventHandler<String>,
) -> Element {
    let active = option.value == selected;
    let class = if active {
        "flex min-w-0 flex-col items-center justify-center rounded-md bg-zinc-800 px-1 py-1 text-center text-white transition-[background-color,color] duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/70"
    } else {
        "flex min-w-0 flex-col items-center justify-center rounded-md px-1 py-1 text-center text-zinc-400 transition-[background-color,color] duration-200 hover:bg-white/[.04] hover:text-zinc-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/70"
    };
    let hint_class = if active {
        "truncate text-[10px] leading-tight text-zinc-400"
    } else {
        "truncate text-[10px] leading-tight text-zinc-600"
    };
    let value = option.value.clone();

    rsx! {
        button {
            key: "{option.value}",
            r#type: "button",
            role: "radio",
            "aria-checked": aria_flag(active),
            class,
            onclick: move |_| on_change.call(value.clone()),
            span { class: "truncate text-[12px] font-semibold leading-tight", "{option.label}" }
            span { class: hint_class, "{option.hint}" }
        }
    }
}

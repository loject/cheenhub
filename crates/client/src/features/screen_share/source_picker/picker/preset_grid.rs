//! Карточки пресетов качества в панели демонстрации экрана.
//!
//! Пресеты — витрина тарифов: они не меняют параметры запуска, поэтому выбор
//! карточки влияет только на её подсветку. Функциональные настройки живут в
//! расширенном блоке панели качества.

use dioxus::prelude::*;

use super::decor::{QUALITY_PRESETS, QualityPreset, aria_flag};

/// Показывает три уровня качества и сообщает индекс выбранной карточки.
#[component]
pub(super) fn PresetGrid(
    selected: Option<usize>,
    subscribed: bool,
    upsell: Option<usize>,
    on_pick: EventHandler<usize>,
) -> Element {
    rsx! {
        div {
            class: "grid grid-cols-3 gap-2",
            role: "radiogroup",
            "aria-label": "Пресеты качества",
            for (index, preset) in QUALITY_PRESETS.iter().enumerate() {
                {preset_card(index, preset, selected == Some(index), upsell == Some(index), preset.requires_subscription && !subscribed, on_pick)}
            }
        }
    }
}

fn preset_card(
    index: usize,
    preset: &'static QualityPreset,
    selected: bool,
    upsell: bool,
    locked: bool,
    on_pick: EventHandler<usize>,
) -> Element {
    let class = if selected {
        "group relative flex flex-col rounded-xl border border-blue-400/50 bg-blue-500/[.10] p-2.5 text-left transition-[border-color,background-color] duration-200 active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/70"
    } else if upsell {
        "group relative flex flex-col rounded-xl border border-amber-300/30 bg-amber-300/[.04] p-2.5 text-left transition-[border-color,background-color] duration-200 active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/70"
    } else {
        "group relative flex flex-col rounded-xl border border-zinc-800 bg-zinc-900/40 p-2.5 text-left transition-[border-color,background-color] duration-200 hover:border-zinc-700 hover:bg-zinc-900 active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/70"
    };
    let icon_class = if selected {
        "grid h-7 w-7 place-items-center rounded-lg bg-blue-400/15 text-blue-200"
    } else if locked {
        "grid h-7 w-7 place-items-center rounded-lg bg-amber-300/10 text-amber-200"
    } else {
        "grid h-7 w-7 place-items-center rounded-lg bg-zinc-800 text-zinc-300"
    };

    rsx! {
        button {
            key: "{index}",
            r#type: "button",
            role: "radio",
            "aria-checked": aria_flag(selected),
            "aria-label": "{preset.name}, {preset.headline}, {preset.meta}",
            class,
            onclick: move |_| on_pick.call(index),
            div { class: "flex items-center justify-between gap-1",
                div { class: icon_class, {preset_icon(index)} }
                if selected {
                    svg { class: "screen-share-pop h-3.5 w-3.5 text-blue-300", fill: "none", stroke: "currentColor", stroke_width: "3", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "m5 12 4 4L19 6" }
                    }
                } else if locked {
                    svg { class: "h-3.5 w-3.5 text-amber-200/70", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                        rect { x: "4", y: "11", width: "16", height: "9", rx: "2" }
                        path { stroke_linecap: "round", d: "M8 11V8a4 4 0 0 1 8 0v3" }
                    }
                }
            }
            p { class: "mt-2 truncate text-[10.5px] font-medium text-zinc-400", "{preset.name}" }
            p { class: "truncate text-[14px] font-semibold leading-tight text-white", "{preset.headline}" }
            p { class: "truncate text-[10px] tabular-nums text-zinc-500", "{preset.meta}" }
        }
    }
}

fn preset_icon(index: usize) -> Element {
    match index {
        0 => rsx! {
            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M13 2 4.5 13.5H11l-1 8.5L19.5 10H13l0-8Z" }
            }
        },
        1 => rsx! {
            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                path { stroke_linecap: "round", d: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z" }
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "m12 12 4.5-4.5" }
            }
        },
        _ => rsx! {
            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 22c4 0 6-2.4 6-5.5 0-4.5-6-14.5-6-14.5S6 12 6 16.5C6 19.6 8 22 12 22Z" }
            }
        },
    }
}

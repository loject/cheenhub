//! Ползунок битрейта в панели качества.
//!
//! Шкала всегда покрывает диапазон до расширенного тарифа, поэтому закрытая
//! зона видна и подписана. Значение выше бесплатного лимита не применяется:
//! бегунок возвращается к границе и сообщает об ограничении тарифа.

use dioxus::prelude::*;

use super::decor::{
    BITRATE_MAX_FREE, BITRATE_MAX_SUBSCRIBED, BITRATE_MIN, BITRATE_TICKS, bitrate_fraction,
    format_bitrate,
};

/// Диаметр бегунка в пикселях, совпадает с [`tailwind.css`].
const THUMB_DIAMETER_PX: f64 = 18.0;

/// Радиус бегунка в пикселях.
const THUMB_RADIUS_PX: f64 = THUMB_DIAMETER_PX / 2.0;

/// Засечка под ползунком битрейта.
struct TickMark {
    /// Значение битрейта в Мбит/с.
    value: f64,
    /// CSS-позиция центра засечки.
    left: String,
    /// Доступна ли засечка без подписки.
    locked: bool,
}

/// Переводит долю шкалы в CSS-позицию центра бегунка.
fn center_position(fraction: f64) -> String {
    format!("calc({fraction} * (100% - {THUMB_DIAMETER_PX}px) + {THUMB_RADIUS_PX}px)")
}

/// Переводит долю шкалы в ширину заливки от центра левого бегунка.
fn fill_width(fraction: f64) -> String {
    format!("calc({fraction} * (100% - {THUMB_DIAMETER_PX}px))")
}

/// Рисует дорожку, заливку, закрытую тарифом зону и засечки вместе с инпутом.
///
/// Инпут покрывает всю шкалу, но значение выше `limit` отклоняется: обработчик
/// возвращает бегунок к границе и сообщает о тарифном ограничении.
#[component]
pub(super) fn BitrateSlider(
    value: f64,
    limit: f64,
    on_change: EventHandler<f64>,
    on_locked_attempt: EventHandler<()>,
) -> Element {
    let fraction = bitrate_fraction(value);
    let fill_left = format!("{THUMB_RADIUS_PX}px");
    let fill = fill_width(fraction);
    let limit_left = center_position(bitrate_fraction(limit));
    let right_offset = format!("{THUMB_RADIUS_PX}px");
    let locked = limit < BITRATE_MAX_SUBSCRIBED;
    let ticks = BITRATE_TICKS
        .iter()
        .copied()
        .map(|tick| TickMark {
            value: tick,
            left: center_position(bitrate_fraction(tick)),
            locked: tick > BITRATE_MAX_FREE,
        })
        .collect::<Vec<_>>();

    rsx! {
        div { class: "px-1 pb-5 pt-1",
            div { class: "relative h-[18px]",
                div { class: "absolute inset-x-0 top-1/2 h-1.5 -translate-y-1/2 rounded-full bg-zinc-800" }
                if locked {
                    div {
                        class: "screen-share-locked-track absolute top-1/2 h-1.5 -translate-y-1/2 rounded-r-full",
                        style: "left: {limit_left}; right: {right_offset};",
                    }
                }
                div {
                    class: "absolute top-1/2 h-1.5 -translate-y-1/2 rounded-full bg-blue-500 transition-[width] duration-150 ease-out",
                    style: "left: {fill_left}; width: {fill};",
                }
                input {
                    r#type: "range",
                    class: "screen-share-range absolute inset-0 w-full",
                    min: "{BITRATE_MIN}",
                    max: "{BITRATE_MAX_SUBSCRIBED}",
                    step: "0.25",
                    value: "{value}",
                    "aria-label": "Видеобитрейт, Мбит/с",
                    oninput: move |event| {
                        let Ok(next) = event.value().parse::<f64>() else {
                            return;
                        };
                        if next > limit {
                            on_change.call(limit);
                            on_locked_attempt.call(());
                        } else {
                            on_change.call(next);
                        }
                    },
                }
                for tick in ticks {
                    {tick_mark(tick)}
                }
            }
        }
    }
}

fn tick_mark(tick: TickMark) -> Element {
    let mark_class = if tick.locked {
        "mx-auto h-1 w-px bg-zinc-700"
    } else {
        "mx-auto h-1 w-px bg-zinc-600"
    };
    let label_class = if tick.locked {
        "mt-0.5 flex items-center gap-0.5 text-[10px] tabular-nums text-zinc-600"
    } else {
        "mt-0.5 text-[10px] tabular-nums text-zinc-500"
    };

    rsx! {
        div { class: "absolute top-full", style: "left: {tick.left};",
            div { class: "-translate-x-1/2",
                div { class: mark_class }
                div { class: label_class,
                    if tick.locked {
                        svg { class: "h-2.5 w-2.5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                            rect { x: "4", y: "11", width: "16", height: "9", rx: "2" }
                            path { stroke_linecap: "round", d: "M8 11V8a4 4 0 0 1 8 0v3" }
                        }
                    }
                    "{format_bitrate(tick.value)}"
                }
            }
        }
    }
}

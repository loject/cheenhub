//! Панель качества демонстрации экрана.
//!
//! Панель позволяет выбрать произвольную комбинацию разрешения и частоты кадров,
//! включая 1440p, 4K и 60 FPS. Битрейт по-прежнему ограничен бесплатным тарифом.
//! Комбинации, которые текущая модель запуска ещё не поддерживает, честно
//! показывают, какое качество будет использовано на самом деле.

use dioxus::prelude::*;

pub(super) use super::decor::QualityChange;

use super::assessment_card::AssessmentCard;
use super::bitrate_slider::BitrateSlider;
use super::decor::{
    BITRATE_MAX_FREE, MockFrameRate, MockResolution, QUALITY_PRESETS, assess, change_for,
    format_bitrate, optimize, preset_index,
};
use super::preset_grid::PresetGrid;
use super::segmented::{Segmented, SegmentedOption};

/// Источник открытого предложения подписки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Upsell {
    /// Выбран тарифный пресет с указанным индексом.
    Preset(usize),
    /// Запрошен битрейт выше бесплатного лимита.
    Bitrate,
}

/// Показывает пресеты, расширенные настройки и итоговую оценку качества.
#[component]
pub(super) fn QualityPanel(
    resolution: MockResolution,
    frame_rate: MockFrameRate,
    on_change: EventHandler<QualityChange>,
) -> Element {
    let mut advanced_open = use_signal(|| false);
    let mut bitrate = use_signal(|| BITRATE_MAX_FREE);
    let mut upsell = use_signal(|| None::<Upsell>);

    let current_preset = preset_index(resolution, frame_rate);
    let assessment = assess(resolution, frame_rate);
    let bitrate_value = bitrate();
    let bitrate_label = format_bitrate(bitrate_value);
    let quality_summary = format!("{} · {}", resolution.label(), frame_rate.label());
    let preset_upsell = matches!(upsell(), Some(Upsell::Preset(_)));
    let bitrate_upsell = matches!(upsell(), Some(Upsell::Bitrate));

    let resolution_options = MockResolution::ALL
        .iter()
        .copied()
        .map(|option| SegmentedOption {
            value: option.value().to_owned(),
            label: option.label().to_owned(),
            hint: option.hint().to_owned(),
        })
        .collect::<Vec<_>>();
    let frame_rate_options = MockFrameRate::ALL
        .iter()
        .copied()
        .map(|option| SegmentedOption {
            value: option.value().to_owned(),
            label: option.label().to_owned(),
            hint: option.hint().to_owned(),
        })
        .collect::<Vec<_>>();

    rsx! {
        section { class: "space-y-2.5", "aria-label": "Качество демонстрации",
            div { class: "flex items-center justify-between gap-2",
                h3 { class: "text-[13.5px] font-semibold tracking-tight text-white", "Качество демонстрации" }
                span { class: "shrink-0 rounded-full border border-zinc-800 bg-white/[.03] px-2 py-0.5 text-[10.5px] font-medium text-zinc-400",
                    "Бесплатный доступ"
                }
            }

            PresetGrid {
                selected: current_preset,
                subscribed: false,
                upsell: match upsell() { Some(Upsell::Preset(index)) => Some(index), _ => None },
                on_pick: move |index: usize| {
                    let preset = QUALITY_PRESETS[index];
                    match preset.apply {
                        Some((mock_resolution, mock_frame_rate)) => {
                            upsell.set(None);
                            on_change.call(change_for(mock_resolution, mock_frame_rate));
                        }
                        None => upsell.set(Some(Upsell::Preset(index))),
                    }
                },
            }

            div { class: "flex h-5 items-center gap-1.5",
                if preset_upsell {
                    span { class: "screen-share-fade-in h-3.5 w-3.5 shrink-0 text-amber-300",
                        svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                            rect { x: "4", y: "11", width: "16", height: "9", rx: "2" }
                            path { stroke_linecap: "round", d: "M8 11V8a4 4 0 0 1 8 0v3" }
                        }
                    }
                    p { class: "screen-share-fade-in min-w-0 flex-1 truncate text-[10.5px] text-zinc-300",
                        "Этот пресет доступен по подписке."
                    }
                    button {
                        r#type: "button",
                        class: "screen-share-fade-in shrink-0 rounded px-1.5 text-[10.5px] font-medium text-zinc-400 transition-colors duration-200 hover:bg-white/[.06] hover:text-zinc-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/70",
                        onclick: move |_| upsell.set(None),
                        "Понятно"
                    }
                }
            }

            div { class: "overflow-hidden rounded-lg border border-zinc-800 bg-white/[.02]",
                button {
                    r#type: "button",
                    "aria-expanded": if advanced_open() { "true" } else { "false" },
                    class: "flex w-full items-center gap-2.5 px-2.5 py-2 text-left transition-colors duration-200 hover:bg-white/[.04] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-blue-300/70",
                    onclick: move |_| advanced_open.toggle(),
                    span { class: "grid h-6 w-6 shrink-0 place-items-center rounded-md bg-white/[.06] text-zinc-300",
                        svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", d: "M4 7h10M18 7h2M4 17h4M12 17h8" }
                            circle { cx: "16", cy: "7", r: "2" }
                            circle { cx: "10", cy: "17", r: "2" }
                        }
                    }
                    span { class: "min-w-0 flex-1",
                        span { class: "flex items-center gap-2",
                            span { class: "text-[12px] font-medium text-zinc-100", "Расширенные настройки" }
                            if current_preset.is_none() {
                                span { class: "rounded-full bg-blue-400/10 px-1.5 py-0.5 text-[9.5px] font-medium text-blue-200",
                                    "Свои"
                                }
                            }
                        }
                        span { class: "mt-0.5 block truncate text-[10.5px] tabular-nums text-zinc-500", "{quality_summary}" }
                    }
                    svg { class: if advanced_open() { "h-3.5 w-3.5 shrink-0 text-zinc-400 transition-transform duration-200 rotate-180" } else { "h-3.5 w-3.5 shrink-0 text-zinc-400 transition-transform duration-200" }, fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "m6 9 6 6 6-6" }
                    }
                }

                if advanced_open() {
                    div { class: "screen-share-reveal border-t border-zinc-800 px-2.5 pb-3 pt-2.5",
                        div { class: "space-y-3",
                            div {
                                h4 { class: "mb-1.5 text-[10.5px] font-medium text-zinc-400", "Разрешение" }
                                Segmented {
                                    label: "Разрешение",
                                    columns: "grid-cols-4",
                                    options: resolution_options,
                                    selected: resolution.value().to_owned(),
                                    on_change: move |value: String| {
                                        if let Some(next) = MockResolution::from_value(&value) {
                                            on_change.call(change_for(next, frame_rate));
                                        }
                                    },
                                }
                            }

                            div {
                                h4 { class: "mb-1.5 text-[10.5px] font-medium text-zinc-400", "Частота кадров" }
                                Segmented {
                                    label: "Частота кадров",
                                    columns: "grid-cols-4",
                                    options: frame_rate_options,
                                    selected: frame_rate.value().to_owned(),
                                    on_change: move |value: String| {
                                        if let Some(next) = MockFrameRate::from_value(&value) {
                                            on_change.call(change_for(resolution, next));
                                        }
                                    },
                                }
                            }

                            div {
                                div { class: "mb-1 flex items-baseline justify-between gap-2",
                                    h4 { class: "text-[10.5px] font-medium text-zinc-400", "Видеобитрейт" }
                                    p { class: if bitrate_upsell { "text-[12px] font-semibold tabular-nums text-amber-200" } else { "text-[12px] font-semibold tabular-nums text-white" },
                                        "{bitrate_label}"
                                        span { class: "ml-1 text-[10px] font-medium text-zinc-500", "Мбит/с" }
                                    }
                                }
                                BitrateSlider {
                                    value: bitrate_value,
                                    limit: BITRATE_MAX_FREE,
                                    on_change: move |value: f64| {
                                        bitrate.set(value);
                                        if value <= BITRATE_MAX_FREE {
                                            upsell.set(None);
                                        }
                                    },
                                    on_locked_attempt: move |_| upsell.set(Some(Upsell::Bitrate)),
                                }
                                if bitrate_upsell {
                                    div { class: "screen-share-fade-in mt-1 flex items-center gap-1.5",
                                        svg { class: "h-3 w-3 shrink-0 text-amber-300", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                                            rect { x: "4", y: "11", width: "16", height: "9", rx: "2" }
                                            path { stroke_linecap: "round", d: "M8 11V8a4 4 0 0 1 8 0v3" }
                                        }
                                        p { class: "min-w-0 flex-1 truncate text-[10.5px] text-amber-200/90",
                                            "Выше {format_bitrate(BITRATE_MAX_FREE)} Мбит/с — по подписке."
                                        }
                                        button {
                                            r#type: "button",
                                            class: "shrink-0 rounded px-1.5 text-[10.5px] font-medium text-zinc-400 transition-colors duration-200 hover:bg-white/[.06] hover:text-zinc-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/70",
                                            onclick: move |_| upsell.set(None),
                                            "Понятно"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            AssessmentCard {
                assessment,
                on_optimize: move |_| on_change.call(optimize()),
            }

            if current_preset.is_none() {
                button {
                    r#type: "button",
                    class: "inline-flex h-6 items-center gap-1.5 rounded text-[10.5px] font-medium text-zinc-500 transition-colors duration-200 hover:text-zinc-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/70",
                    onclick: move |_| {
                        if let Some((mock_resolution, mock_frame_rate)) = QUALITY_PRESETS[0].apply {
                            on_change.call(change_for(mock_resolution, mock_frame_rate));
                        }
                    },
                    svg { class: "h-3 w-3", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3 12a9 9 0 1 0 3-6.7M3 4v5h5" }
                    }
                    "Сбросить до «Стандартное»"
                }
            }
        }
    }
}

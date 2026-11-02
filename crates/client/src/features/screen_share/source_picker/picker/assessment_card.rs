//! Итоговая подсказка о выбранных параметрах демонстрации.

use dioxus::prelude::*;

use super::decor::{AssessmentTone, QualityAssessment};

/// Показывает оценку сочетания разрешения и частоты кадров.
///
/// Для неоптимальных сочетаний доступно действие, подбирающее параметры
/// автоматически.
#[component]
pub(super) fn AssessmentCard(
    assessment: QualityAssessment,
    on_optimize: EventHandler<()>,
) -> Element {
    let good = assessment.tone == AssessmentTone::Good;
    let icon_class = if good {
        "mt-0.5 h-4 w-4 shrink-0 text-emerald-300"
    } else {
        "mt-0.5 h-4 w-4 shrink-0 text-blue-300"
    };
    let bar_class = if good {
        "bg-emerald-400"
    } else {
        "bg-blue-400"
    };

    rsx! {
        div { key: "{assessment.title}", class: "screen-share-fade-in rounded-xl border border-zinc-800 bg-zinc-900/40 p-3", role: "status", "aria-live": "polite",
            div { class: "flex items-start gap-2.5",
                svg { class: icon_class, fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z" }
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "m8.5 12.5 2.5 2.5 4.5-5" }
                }
                div { class: "min-w-0 flex-1",
                    div { class: "flex items-center justify-between gap-3",
                        p { class: "truncate text-[12.5px] font-semibold text-zinc-100", "{assessment.title}" }
                        div { class: "flex shrink-0 items-end gap-[3px]", "aria-hidden": "true",
                            for step in 1..=4u8 {
                                div {
                                    key: "{step}",
                                    class: if step <= assessment.score {
                                        "w-[3px] rounded-full {bar_class}"
                                    } else {
                                        "w-[3px] rounded-full bg-zinc-700"
                                    },
                                    style: "height: {(5 + step * 3)}px;",
                                }
                            }
                        }
                    }
                    p { class: "mt-0.5 text-[11.5px] leading-relaxed text-zinc-400", "{assessment.text}" }
                    if !good {
                        button {
                            r#type: "button",
                            class: "mt-2 inline-flex min-h-8 items-center gap-1.5 rounded-lg bg-white/[.06] px-2.5 text-[11.5px] font-medium text-zinc-100 transition-[background-color] duration-200 hover:bg-white/[.10] active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/70",
                            onclick: move |_| on_optimize.call(()),
                            svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "m4 20 9-9M14 4l1.5 3L19 8.5 15.5 10 14 13l-1.5-3L9 8.5 12.5 7 14 4Z" }
                            }
                            "Подобрать оптимальные"
                        }
                    }
                }
            }
        }
    }
}

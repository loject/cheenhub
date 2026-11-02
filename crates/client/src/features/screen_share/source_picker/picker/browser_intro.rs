//! Пояснение для браузерного режима выбора источника.

use dioxus::prelude::*;

/// Шаги выбора источника в системном окне браузера.
static STEPS: [(&str, &str); 3] = [
    (
        "Настройте качество",
        "Выберите пресет или задайте параметры вручную.",
    ),
    (
        "Нажмите «Выбрать экран и начать»",
        "Браузер покажет собственное окно выбора.",
    ),
    (
        "Выберите, что показать",
        "Экран, окно или вкладку — демонстрация начнётся сразу.",
    ),
];

/// Объясняет, почему источник выбирается средствами браузера.
#[component]
pub(super) fn BrowserIntro() -> Element {
    rsx! {
        div { class: "rounded-lg border border-zinc-800 bg-zinc-900/40 p-4",
            div { class: "flex gap-3",
                div { class: "grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-blue-500/15 text-blue-300",
                    svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                        circle { cx: "12", cy: "12", r: "9" }
                        path { stroke_linecap: "round", d: "M3 12h18M12 3c2.5 2.6 2.5 15.4 0 18M12 3c-2.5 2.6-2.5 15.4 0 18" }
                    }
                }
                div { class: "min-w-0",
                    h3 { class: "text-[13.5px] font-semibold tracking-tight text-white", "Источник выбирается в браузере" }
                    p { class: "mt-1 text-[12px] leading-relaxed text-zinc-400",
                        "Браузер сам спросит, что именно показывать: так устроены права доступа к экрану в вебе."
                    }
                }
            }
            ol { class: "mt-3 space-y-1.5",
                for (index, step) in STEPS.iter().enumerate() {
                    li { key: "{index}", class: "flex items-start gap-2 rounded-md bg-zinc-950/40 px-2.5 py-2",
                        span { class: "grid h-4 w-4 shrink-0 place-items-center rounded-full bg-zinc-800 text-[10px] font-semibold text-zinc-300", "{index + 1}" }
                        div { class: "min-w-0",
                            p { class: "text-[11.5px] font-medium leading-tight text-zinc-100", "{step.0}" }
                            p { class: "mt-0.5 text-[11px] leading-relaxed text-zinc-500", "{step.1}" }
                        }
                    }
                }
            }
        }
    }
}

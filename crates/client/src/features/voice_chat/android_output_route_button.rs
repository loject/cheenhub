//! Кнопка выбора системного Android-маршрута вывода голосового звонка.

use dioxus::prelude::*;

use super::voice_call_platform::{self, VoiceOutputRoute};

/// Возвращает, может ли текущая платформа показать переключатель маршрута звука.
pub(super) fn output_route_control_available() -> bool {
    voice_call_platform::supports_voice_output_route()
}

/// Показывает переключатель громкой связи, когда платформа и устройство его поддерживают.
#[component]
pub(super) fn AndroidOutputRouteButton(enabled: bool) -> Element {
    if !output_route_control_available() {
        return rsx! {};
    }

    let mut route_load = use_resource(voice_call_platform::load_voice_output_route);
    let mut selected_route = use_signal(|| None::<VoiceOutputRoute>);
    let mut switching = use_signal(|| false);
    let mut switch_error = use_signal(|| None::<String>);
    let loaded_route = route_load.read().clone();

    match loaded_route {
        None => loading_button("Определяем режим звука"),
        Some(Ok(None)) => rsx! {},
        Some(Err(_error)) => {
            rsx! {
                div { class: "relative flex shrink-0",
                    button {
                        r#type: "button",
                        class: "voice-control-button direct-call-control group relative flex h-12 w-12 items-center justify-center rounded-xl bg-amber-500/12 text-amber-100 shadow-[0_0_0_1px_rgba(245,158,11,.25)] transition-[scale,background-color,box-shadow,opacity] duration-[180ms] active:scale-[0.96]",
                        "aria-label": "Повторить определение режима звука",
                        onclick: move |_| route_load.restart(),
                        {tooltip("Повторить определение режима звука")}
                        {warning_icon()}
                    }
                    {route_error("Не удалось определить режим звука. Нажми ещё раз.")}
                }
            }
        }
        Some(Ok(Some(initial_route))) => {
            let route = selected_route().unwrap_or(initial_route);
            let speaker_enabled = route == VoiceOutputRoute::Speaker;
            let label = if speaker_enabled {
                "Переключить на тихий звук"
            } else {
                "Включить громкую связь"
            };
            let pending = switching();
            let disabled = !enabled || pending;
            let error = switch_error();
            let button_label = if error.is_some() {
                "Повторить переключение режима звука"
            } else if pending {
                "Переключаем режим звука"
            } else {
                label
            };
            rsx! {
                div { class: "relative flex shrink-0",
                    button {
                        r#type: "button",
                        disabled,
                        class: if speaker_enabled {
                            "voice-control-button direct-call-control group relative flex h-12 w-12 items-center justify-center rounded-xl bg-blue-500/16 text-blue-100 shadow-[0_0_0_1px_rgba(96,165,250,.30)] transition-[scale,background-color,box-shadow,opacity] duration-[180ms] active:scale-[0.96] disabled:cursor-wait disabled:opacity-60"
                        } else {
                            "voice-control-button direct-call-control group relative flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-950/65 text-zinc-200 shadow-[0_0_0_1px_rgba(255,255,255,.08)] transition-[scale,background-color,box-shadow,opacity] duration-[180ms] active:scale-[0.96] disabled:cursor-wait disabled:opacity-60"
                        },
                        "aria-label": button_label,
                        onclick: move |_| {
                            if disabled {
                                return;
                            }
                            let next_route = if speaker_enabled {
                                VoiceOutputRoute::Earpiece
                            } else {
                                VoiceOutputRoute::Speaker
                            };
                            switching.set(true);
                            switch_error.set(None);
                            spawn(async move {
                                match voice_call_platform::set_voice_output_route(next_route).await {
                                    Ok(()) => selected_route.set(Some(next_route)),
                                    Err(error) => {
                                        warn!(%error, ?next_route, "failed to switch Android voice output route");
                                        switch_error.set(Some(error.to_string()));
                                    }
                                }
                                switching.set(false);
                            });
                        },
                        {tooltip(label)}
                        {speaker_icon(speaker_enabled && !pending)}
                        {earpiece_icon(!speaker_enabled && !pending)}
                        if pending {
                            {loading_spinner()}
                        }
                    }
                    if error.is_some() {
                        {route_error("Не удалось переключить звук. Нажми ещё раз.")}
                    }
                }
            }
        }
    }
}

fn loading_button(label: &'static str) -> Element {
    rsx! {
        button {
            r#type: "button",
            disabled: true,
            class: "voice-control-button direct-call-control group relative flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-950/65 text-zinc-300 opacity-60 shadow-[0_0_0_1px_rgba(255,255,255,.08)] transition-[scale,background-color,box-shadow,opacity] duration-[180ms] disabled:cursor-wait",
            "aria-label": label,
            {tooltip(label)}
            {loading_spinner()}
        }
    }
}

fn route_error(message: &str) -> Element {
    rsx! {
        span {
            role: "alert",
            class: "voice-output-route-error pointer-events-none absolute bottom-[calc(100%+12px)] left-1/2 z-10 w-max max-w-[min(18rem,calc(100vw-2rem))] -translate-x-1/2 rounded-xl border border-amber-500/35 bg-zinc-950 px-3 py-2 text-center text-[12px] font-medium leading-4 text-amber-100 shadow-[0_12px_32px_rgba(0,0,0,.36)]",
            "{message}"
        }
    }
}

fn tooltip(label: &str) -> Element {
    rsx! {
        span {
            class: "pointer-events-none absolute bottom-[calc(100%+10px)] left-1/2 -translate-x-1/2 translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100",
            "{label}"
        }
    }
}

fn loading_spinner() -> Element {
    rsx! {
        span {
            class: "voice-control-state-icon h-5 w-5 animate-spin rounded-full border-2 border-zinc-600 border-t-blue-300",
            "aria-hidden": "true",
        }
    }
}

fn warning_icon() -> Element {
    rsx! {
        svg {
            class: "voice-control-state-icon h-5 w-5",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.9",
            view_box: "0 0 24 24",
            "aria-hidden": "true",
            path {
                stroke_linecap: "round",
                stroke_linejoin: "round",
                d: "M12 9v4m0 4h.01M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z",
            }
        }
    }
}

fn speaker_icon(visible: bool) -> Element {
    rsx! {
        span {
            class: if visible {
                "voice-control-state-icon absolute grid scale-100 place-items-center opacity-100 blur-0 transition-[transform,opacity,filter] duration-300 ease-[cubic-bezier(0.2,0,0,1)]"
            } else {
                "voice-control-state-icon absolute grid scale-[0.25] place-items-center opacity-0 blur-[4px] transition-[transform,opacity,filter] duration-300 ease-[cubic-bezier(0.2,0,0,1)]"
            },
            "aria-hidden": "true",
            svg {
                class: "h-5 w-5",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.9",
                view_box: "0 0 24 24",
                path {
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    d: "M11 5 6 9H3v6h3l5 4V5Zm4.5 4.5a4 4 0 0 1 0 5m2.5-8a7.5 7.5 0 0 1 0 11",
                }
            }
        }
    }
}

fn earpiece_icon(visible: bool) -> Element {
    rsx! {
        span {
            class: if visible {
                "voice-control-state-icon absolute grid scale-100 place-items-center opacity-100 blur-0 transition-[transform,opacity,filter] duration-300 ease-[cubic-bezier(0.2,0,0,1)]"
            } else {
                "voice-control-state-icon absolute grid scale-[0.25] place-items-center opacity-0 blur-[4px] transition-[transform,opacity,filter] duration-300 ease-[cubic-bezier(0.2,0,0,1)]"
            },
            "aria-hidden": "true",
            svg {
                class: "h-5 w-5",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.9",
                view_box: "0 0 24 24",
                rect { x: "7", y: "3", width: "10", height: "18", rx: "2.5" }
                path {
                    stroke_linecap: "round",
                    d: "M10 6h4m-3 12h2",
                }
            }
        }
    }
}

//! User context menu component.

use dioxus::prelude::*;

/// Renders a fixed-position user context menu.
#[component]
pub(crate) fn UserContextMenu(
    name: String,
    is_self: bool,
    volume: u32,
    x: f64,
    y: f64,
    on_volume_change: EventHandler<u32>,
) -> Element {
    let top = y + 8.0;
    let pos_style = format!(
        "left: clamp(12px, {x}px, calc(100vw - 258px)); top: clamp(12px, {top}px, calc(100vh - 240px));"
    );
    let fill = (volume as f64) / 2.0;
    let slider_bg = format!(
        "background: linear-gradient(to right, #3b82f6 {fill:.1}%, #27272a {fill:.1}%);"
    );
    let muted = volume == 0;

    rsx! {
        div {
            class: "user-menu fixed z-[1000] w-[240px] rounded-[18px] border border-zinc-800 bg-zinc-950/96 p-1.5 shadow-[0_20px_60px_rgba(0,0,0,.65)] backdrop-blur-xl",
            style: pos_style,
            onclick: move |event| event.stop_propagation(),

            // Header
            div { class: "flex items-center gap-2.5 rounded-[12px] px-2.5 py-2",
                div { class: "flex h-7 w-7 shrink-0 items-center justify-center rounded-full border border-zinc-700/60 bg-zinc-800 text-[11px] font-bold text-zinc-300",
                    { name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default() }
                }
                div { class: "min-w-0 flex-1",
                    div { class: "truncate text-[13px] font-semibold leading-tight text-zinc-100", "{name}" }
                    div { class: "text-[11px] leading-tight text-zinc-600",
                        if is_self { "Это вы" } else { "Участник" }
                    }
                }
            }

            if !is_self {
                div { class: "mx-1 my-1 border-t border-zinc-800/70" }

                // Volume row
                div { class: "px-2.5 pt-2 pb-2.5",
                    div { class: "mb-2.5 flex items-center justify-between",
                        span { class: "text-[11px] font-medium text-zinc-500", "Громкость" }
                        // Clicking the badge resets volume to 100%
                        button {
                            r#type: "button",
                            title: "Сбросить до 100%",
                            class: if muted {
                                "rounded-md bg-zinc-900 px-1.5 py-0.5 text-[10px] font-semibold tabular-nums text-zinc-600 transition-[background,color] duration-100 hover:bg-zinc-800 hover:text-zinc-400"
                            } else {
                                "rounded-md bg-blue-500/10 px-1.5 py-0.5 text-[10px] font-semibold tabular-nums text-blue-400 transition-[background,color] duration-100 hover:bg-blue-500/20 hover:text-blue-300"
                            },
                            onclick: move |_| on_volume_change.call(100),
                            "{volume}%"
                        }
                    }
                    div { class: "flex items-center gap-2",
                        // Speaker icon — click to toggle mute/unmute
                        button {
                            r#type: "button",
                            title: if muted { "Включить звук" } else { "Выключить звук" },
                            class: if muted {
                                "rounded-lg p-0.5 text-zinc-600 transition-[color] duration-100 hover:text-zinc-400"
                            } else {
                                "rounded-lg p-0.5 text-zinc-500 transition-[color] duration-100 hover:text-zinc-300"
                            },
                            onclick: move |_| on_volume_change.call(if muted { 100 } else { 0 }),
                            svg {
                                class: "h-3.5 w-3.5",
                                fill: "none", stroke: "currentColor", stroke_width: "1.9",
                                view_box: "0 0 24 24", "aria-hidden": "true",
                                if muted {
                                    path { stroke_linecap: "round", stroke_linejoin: "round",
                                        d: "M17.25 9.75 19.5 12m0 0 2.25 2.25M19.5 12l2.25-2.25M19.5 12l-2.25 2.25m-10.5-6 4.72-4.72a.75.75 0 0 1 1.28.53v15.88a.75.75 0 0 1-1.28.53l-4.72-4.72H4.51c-.88 0-1.704-.507-1.938-1.354A9.009 9.009 0 0 1 2.25 12c0-.83.112-1.633.322-2.396C2.806 8.756 3.63 8.25 4.51 8.25H6.75Z"
                                    }
                                } else {
                                    path { stroke_linecap: "round", stroke_linejoin: "round",
                                        d: "M19.114 5.636a9 9 0 0 1 0 12.728M16.463 8.288a5.25 5.25 0 0 1 0 7.424M6.75 8.25l4.72-4.72a.75.75 0 0 1 1.28.53v15.88a.75.75 0 0 1-1.28.53l-4.72-4.72H4.51c-.88 0-1.704-.507-1.938-1.354A9.01 9.01 0 0 1 2.25 12c0-.83.112-1.633.322-2.396C2.806 8.756 3.63 8.25 4.51 8.25H6.75Z"
                                    }
                                }
                            }
                        }
                        input {
                            class: "volume-slider flex-1",
                            r#type: "range",
                            min: "0",
                            max: "200",
                            value: volume,
                            style: slider_bg,
                            oninput: move |event| {
                                if let Ok(val) = event.value().parse::<u32>() {
                                    on_volume_change.call(val);
                                }
                            },
                        }
                    }
                }

                div { class: "mx-1 my-1 border-t border-zinc-800/70" }

                button {
                    r#type: "button",
                    class: "flex w-full items-center justify-between rounded-[10px] px-2.5 py-2 text-left text-[13px] text-zinc-300 transition-[background,color] duration-100 hover:bg-zinc-900 hover:text-zinc-100",
                    span { "Кикнуть из голоса" }
                    span { class: "text-[10px] text-zinc-700", "админ" }
                }
                button {
                    r#type: "button",
                    class: "flex w-full items-center justify-between rounded-[10px] px-2.5 py-2 text-left text-[13px] text-red-400/80 transition-[background,color] duration-100 hover:bg-red-500/10 hover:text-red-300",
                    span { "Кикнуть с сервера" }
                    span { class: "text-[10px] text-red-500/40", "админ" }
                }
            }
        }
    }
}

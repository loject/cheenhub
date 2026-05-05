//! Voice participant stage component.

use dioxus::prelude::*;

use super::user_context_menu::UserContextMenu;

#[derive(Clone, Copy, PartialEq)]
struct UserMenuState {
    name: &'static str,
    volume: &'static str,
    x: f64,
    y: f64,
}

/// Renders the static voice participant grid.
#[component]
pub(crate) fn VoiceStage() -> Element {
    let mut open_user_menu = use_signal(|| None::<UserMenuState>);

    rsx! {
        div {
            class: "voice-stage flex min-h-0 flex-1 items-center justify-center overflow-y-auto p-5 pt-6 pb-[108px] max-[900px]:pt-4 max-[900px]:pb-[100px] lg:p-6 lg:pt-6 lg:pb-[108px]",
            onclick: move |_| open_user_menu.set(None),
            div { class: "participants-grid m-auto flex min-h-full w-[min(100%,1320px)] flex-wrap content-center items-stretch justify-center gap-4 max-[900px]:gap-3", "data-count": "5",
                article {
                    "data-avatar": "Ч",
                    style: "--avatar-bg: rgba(24,24,27,.80);",
                    class: "user-tile relative overflow-hidden rounded-[20px] border border-accent/30 bg-[var(--avatar-bg,rgba(24,24,27,.8))] bg-cover bg-center p-4 transition-[border-color,background,transform] duration-200 ease-in-out hover:border-white/15 animate-[soft-pulse_1.8s_ease-in-out_infinite]",
                    oncontextmenu: move |event| {
                        event.prevent_default();
                        event.stop_propagation();
                        let point = event.client_coordinates();
                        open_user_menu.set(Some(UserMenuState { name: "chingiz", volume: "100", x: point.x, y: point.y }));
                    },
                    div { class: "absolute right-3 top-3 z-20",
                        button {
                            r#type: "button",
                            class: "transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px rounded-xl border border-zinc-800 bg-zinc-950 p-2 text-zinc-500 hover:border-zinc-700 hover:text-zinc-200",
                            "aria-label": "Меню пользователя",
                            onclick: move |event| {
                                event.stop_propagation();
                                let point = event.client_coordinates();
                                open_user_menu.set(Some(UserMenuState { name: "chingiz", volume: "100", x: point.x, y: point.y }));
                            },
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6.75 12a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Z" }
                            }
                        }
                    }
                    div { class: "relative z-10 flex min-h-full flex-col justify-end text-left",
                        div { class: "flex items-center gap-2",
                            div { class: "flex h-5 items-end gap-0.5 text-accent", "aria-label": "Говорит",
                                span { class: "inline-block h-3.5 w-[3px] origin-bottom animate-[voice-pulse-bar_.82s_ease-in-out_infinite] rounded-full bg-current" }
                                span { class: "inline-block h-[18px] w-[3px] origin-bottom animate-[voice-pulse-bar_.82s_ease-in-out_infinite] rounded-full bg-current [animation-delay:.12s]" }
                                span { class: "inline-block h-3 w-[3px] origin-bottom animate-[voice-pulse-bar_.82s_ease-in-out_infinite] rounded-full bg-current [animation-delay:.24s]" }
                                span { class: "inline-block h-4 w-[3px] origin-bottom animate-[voice-pulse-bar_.82s_ease-in-out_infinite] rounded-full bg-current [animation-delay:.36s]" }
                            }
                            div { class: "text-[14px] font-semibold tracking-[-0.03em] text-zinc-50", "chingiz" }
                        }
                    }
                }
                article {
                    "data-avatar": "В",
                    style: "--avatar-bg: rgba(24,24,27,.80);",
                    class: "user-tile relative overflow-hidden rounded-[20px] border border-accent/25 bg-[var(--avatar-bg,rgba(24,24,27,.8))] bg-cover bg-center p-4 transition-[border-color,background,transform] duration-200 ease-in-out hover:border-white/15 animate-[soft-pulse_1.8s_ease-in-out_infinite]",
                    oncontextmenu: move |event| {
                        event.prevent_default();
                        event.stop_propagation();
                        let point = event.client_coordinates();
                        open_user_menu.set(Some(UserMenuState { name: "valeria", volume: "86", x: point.x, y: point.y }));
                    },
                    div { class: "absolute right-3 top-3 z-20",
                        button {
                            r#type: "button",
                            class: "transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px rounded-xl border border-zinc-800 bg-zinc-950 p-2 text-zinc-500 hover:border-zinc-700 hover:text-zinc-200",
                            "aria-label": "Меню пользователя",
                            onclick: move |event| {
                                event.stop_propagation();
                                let point = event.client_coordinates();
                                open_user_menu.set(Some(UserMenuState { name: "valeria", volume: "86", x: point.x, y: point.y }));
                            },
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6.75 12a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Z" }
                            }
                        }
                    }
                    div { class: "absolute bottom-3 right-3 flex gap-1.5",
                        span { class: "rounded-lg border border-zinc-700 bg-zinc-950/80 p-1.5 text-zinc-300", title: "Демонстрация экрана",
                            svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                                rect { x: "3", y: "4", width: "18", height: "12", rx: "2" }
                                path { stroke_linecap: "round", d: "M8 20h8m-4-4v4" }
                            }
                        }
                    }
                    div { class: "relative z-10 flex min-h-full flex-col justify-end text-left",
                        div { class: "flex items-center gap-2",
                            div { class: "flex h-5 items-end gap-0.5 text-accent", "aria-label": "Говорит",
                                span { class: "inline-block h-3.5 w-[3px] origin-bottom animate-[voice-pulse-bar_.82s_ease-in-out_infinite] rounded-full bg-current" }
                                span { class: "inline-block h-[18px] w-[3px] origin-bottom animate-[voice-pulse-bar_.82s_ease-in-out_infinite] rounded-full bg-current [animation-delay:.12s]" }
                                span { class: "inline-block h-3 w-[3px] origin-bottom animate-[voice-pulse-bar_.82s_ease-in-out_infinite] rounded-full bg-current [animation-delay:.24s]" }
                                span { class: "inline-block h-4 w-[3px] origin-bottom animate-[voice-pulse-bar_.82s_ease-in-out_infinite] rounded-full bg-current [animation-delay:.36s]" }
                            }
                            div { class: "text-[14px] font-semibold tracking-[-0.03em] text-zinc-50", "valeria" }
                        }
                    }
                }
                article {
                    "data-avatar": "A",
                    style: "--avatar-bg: rgba(24,24,27,.80);",
                    class: "user-tile relative overflow-hidden rounded-[20px] border border-zinc-800 bg-[var(--avatar-bg,rgba(24,24,27,.8))] bg-cover bg-center p-4 transition-[border-color,background,transform] duration-200 ease-in-out hover:border-white/15",
                    oncontextmenu: move |event| {
                        event.prevent_default();
                        event.stop_propagation();
                        let point = event.client_coordinates();
                        open_user_menu.set(Some(UserMenuState { name: "alex", volume: "100", x: point.x, y: point.y }));
                    },
                    div { class: "absolute right-3 top-3 z-20",
                        button {
                            r#type: "button",
                            class: "transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px rounded-xl border border-zinc-800 bg-zinc-950 p-2 text-zinc-500 hover:border-zinc-700 hover:text-zinc-200",
                            "aria-label": "Меню пользователя",
                            onclick: move |event| {
                                event.stop_propagation();
                                let point = event.client_coordinates();
                                open_user_menu.set(Some(UserMenuState { name: "alex", volume: "100", x: point.x, y: point.y }));
                            },
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6.75 12a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Z" }
                            }
                        }
                    }
                    div { class: "relative z-10 flex min-h-full flex-col justify-end text-left",
                        div { class: "flex items-center gap-2",
                            span { class: "rounded-lg border border-zinc-800 bg-zinc-950/80 p-1.5 text-zinc-500", title: "Микрофон выключен",
                                svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 9.34V7a3 3 0 0 0-5.68-1.34M9 9v2a3 3 0 0 0 5.12 2.12M19 11a7 7 0 0 1-7 7m0 0v3m0-3a7 7 0 0 1-7-7m3 10h8M3 3l18 18" }
                                }
                            }
                            div { class: "text-[14px] font-semibold tracking-[-0.03em] text-zinc-100", "alex" }
                        }
                    }
                }
                article {
                    "data-avatar": "M",
                    style: "--avatar-bg: rgba(24,24,27,.80);",
                    class: "user-tile relative overflow-hidden rounded-[20px] border border-zinc-800 bg-[var(--avatar-bg,rgba(24,24,27,.8))] bg-cover bg-center p-4 transition-[border-color,background,transform] duration-200 ease-in-out hover:border-white/15",
                    oncontextmenu: move |event| {
                        event.prevent_default();
                        event.stop_propagation();
                        let point = event.client_coordinates();
                        open_user_menu.set(Some(UserMenuState { name: "mira", volume: "120", x: point.x, y: point.y }));
                    },
                    div { class: "absolute right-3 top-3 z-20",
                        button {
                            r#type: "button",
                            class: "transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px rounded-xl border border-zinc-800 bg-zinc-950 p-2 text-zinc-500 hover:border-zinc-700 hover:text-zinc-200",
                            "aria-label": "Меню пользователя",
                            onclick: move |event| {
                                event.stop_propagation();
                                let point = event.client_coordinates();
                                open_user_menu.set(Some(UserMenuState { name: "mira", volume: "120", x: point.x, y: point.y }));
                            },
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6.75 12a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Z" }
                            }
                        }
                    }
                    div { class: "relative z-10 flex min-h-full flex-col justify-end text-left",
                        div { class: "flex items-center gap-2",
                            div { class: "text-[14px] font-semibold tracking-[-0.03em] text-zinc-100", "mira" }
                        }
                    }
                }
                article {
                    "data-avatar": "D",
                    style: "--avatar-bg: rgba(24,24,27,.80);",
                    class: "user-tile relative overflow-hidden rounded-[20px] border border-zinc-800 bg-[var(--avatar-bg,rgba(24,24,27,.8))] bg-cover bg-center p-4 opacity-90 transition-[border-color,background,transform] duration-200 ease-in-out hover:border-white/15",
                    oncontextmenu: move |event| {
                        event.prevent_default();
                        event.stop_propagation();
                        let point = event.client_coordinates();
                        open_user_menu.set(Some(UserMenuState { name: "den", volume: "100", x: point.x, y: point.y }));
                    },
                    div { class: "absolute bottom-3 right-3 rounded-lg border border-zinc-800 bg-zinc-950/80 p-1.5 text-zinc-500", title: "Камера включена",
                        svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "m15.75 10.5 4.72-4.72a.75.75 0 0 1 1.28.53v11.38a.75.75 0 0 1-1.28.53l-4.72-4.72M4.5 18.75h9a2.25 2.25 0 0 0 2.25-2.25v-9A2.25 2.25 0 0 0 13.5 5.25h-9A2.25 2.25 0 0 0 2.25 7.5v9a2.25 2.25 0 0 0 2.25 2.25Z" }
                        }
                    }
                    div { class: "absolute right-3 top-3 z-20",
                        button {
                            r#type: "button",
                            class: "transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px rounded-xl border border-zinc-800 bg-zinc-950 p-2 text-zinc-500 hover:border-zinc-700 hover:text-zinc-200",
                            "aria-label": "Меню пользователя",
                            onclick: move |event| {
                                event.stop_propagation();
                                let point = event.client_coordinates();
                                open_user_menu.set(Some(UserMenuState { name: "den", volume: "100", x: point.x, y: point.y }));
                            },
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6.75 12a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Z" }
                            }
                        }
                    }
                    div { class: "relative z-10 flex min-h-full flex-col justify-end text-left",
                        div { class: "flex items-center gap-2",
                            span { class: "rounded-lg border border-zinc-800 bg-zinc-950/80 p-1.5 text-zinc-500", title: "Микрофон выключен",
                                svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 9.34V7a3 3 0 0 0-5.68-1.34M9 9v2a3 3 0 0 0 5.12 2.12M19 11a7 7 0 0 1-7 7m0 0v3m0-3a7 7 0 0 1-7-7m3 10h8M3 3l18 18" }
                                }
                            }
                            div { class: "text-[14px] font-semibold tracking-[-0.03em] text-zinc-100", "den" }
                        }
                    }
                }
            }
            if let Some(menu) = open_user_menu() {
                UserContextMenu {
                    name: menu.name,
                    volume: menu.volume,
                    x: menu.x,
                    y: menu.y,
                }
            }
        }
    }
}

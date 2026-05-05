//! Room list and local voice status sidebar.

use cheenhub_contracts::rest::ServerSummary;
use dioxus::prelude::*;

use super::app_shell::ActiveRoom;
use super::server_context_menu::ServerContextMenu;

/// Renders the static server room sidebar.
#[component]
pub(crate) fn RoomSidebar(
    server: ServerSummary,
    active_room: &'static str,
    on_create_invite: EventHandler<String>,
    on_select_room: EventHandler<ActiveRoom>,
) -> Element {
    let mut is_server_menu_open = use_signal(|| false);
    let server_name = server.name.clone();
    let invite_server_name = server_name.clone();
    let is_owner = server.is_owner;

    rsx! {
        aside {
            class: "flex w-[284px] shrink-0 flex-col border-r border-zinc-800/80 bg-zinc-950/85 backdrop-blur-xl",
            onclick: move |_| is_server_menu_open.set(false),
            div { class: "relative border-b border-zinc-800/80 p-4",
                button {
                    r#type: "button",
                    class: "flex w-full items-center justify-between rounded-2xl border border-zinc-800 bg-zinc-900/80 px-4 py-3 text-left transition-[background,border-color,color,transform,opacity] duration-150 hover:border-zinc-700 hover:bg-zinc-800",
                    "aria-haspopup": "menu",
                    "aria-expanded": if is_server_menu_open() { "true" } else { "false" },
                    onclick: move |event| {
                        event.stop_propagation();
                        is_server_menu_open.set(!is_server_menu_open());
                    },
                    span {
                        span { class: "block text-[13px] font-semibold tracking-[-0.02em] text-zinc-100", "{server_name}" }
                        span { class: "mt-0.5 block text-[11px] text-zinc-500", "Сервер разработки · 18 онлайн" }
                    }
                    svg { class: "h-4 w-4 text-zinc-500", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "m6 9 6 6 6-6" }
                    }
                }
                if is_server_menu_open() {
                    ServerContextMenu {
                        is_owner,
                        on_create_invite: move |_| {
                            is_server_menu_open.set(false);
                            on_create_invite.call(invite_server_name.clone());
                        },
                    }
                }
            }

            div { class: "min-h-0 flex-1 overflow-y-auto p-3",
                div { class: "mb-1.5 flex items-center justify-between px-1 text-[10px] font-medium uppercase tracking-[0.22em] text-zinc-600",
                    span { "Комнаты" }
                    button { r#type: "button", class: "rounded-md p-1 text-zinc-600 hover:bg-zinc-900 hover:text-zinc-300", "aria-label": "Создать комнату",
                        svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 5v14m-7-7h14" }
                        }
                    }
                }
                div { class: "space-y-1",
                    div { "data-active": if active_room == "Лобби" { "true" } else { "false" }, class: "relative flex w-full items-center justify-between rounded-lg border border-transparent px-2.5 py-2 text-left text-zinc-400 transition-[background,border-color,color,transform,opacity] duration-150 hover:border-zinc-800 hover:bg-zinc-900 hover:text-zinc-100 data-[active=true]:border-accent/25 data-[active=true]:bg-accent/10 data-[active=true]:text-zinc-100",
                        button {
                            r#type: "button",
                            class: "flex min-w-0 flex-1 items-center gap-2 text-left",
                            "aria-label": "Открыть комнату Лобби",
                            onclick: move |_| on_select_room.call(ActiveRoom { kind: "mixed", name: "Лобби" }),
                            span { class: "w-3.5 shrink-0 text-center text-[13px] font-semibold leading-none text-accent", "&" }
                            span { class: "truncate text-[12px] font-medium", "Лобби" }
                        }
                        span { class: "flex shrink-0 items-center -space-x-1.5 pl-2",
                            span { class: "flex h-5 w-5 items-center justify-center rounded-full border border-zinc-950 bg-zinc-100 text-[9px] font-bold text-zinc-950", "aria-label": "chingiz", "Ч" }
                            span { class: "flex h-5 w-5 items-center justify-center rounded-full border border-zinc-950 bg-zinc-800 text-[9px] font-bold text-zinc-100", "aria-label": "valeria", "В" }
                            span { class: "flex h-5 w-5 items-center justify-center rounded-full border border-zinc-950 bg-zinc-800 text-[9px] font-bold text-zinc-100", "aria-label": "alex", "A" }
                            button { r#type: "button", class: "relative z-10 flex h-5 min-w-5 items-center justify-center rounded-full border border-zinc-950 bg-zinc-900 px-1 text-[9px] font-semibold text-zinc-400 transition hover:border-accent/35 hover:bg-accent/10 hover:text-zinc-100", "aria-label": "Показать всех участников Лобби", "+2" }
                        }
                    }
                    button { r#type: "button", "data-active": if active_room == "Разработка" { "true" } else { "false" }, class: "flex w-full items-center justify-between rounded-lg border border-transparent px-2.5 py-2 text-left text-zinc-400 transition-[background,border-color,color,transform,opacity] duration-150 hover:border-zinc-800 hover:bg-zinc-900 hover:text-zinc-100 data-[active=true]:border-accent/25 data-[active=true]:bg-accent/10 data-[active=true]:text-zinc-100",
                        onclick: move |_| on_select_room.call(ActiveRoom { kind: "mixed-chat", name: "Разработка" }),
                        span { class: "flex min-w-0 items-center gap-2",
                            span { class: "w-3.5 shrink-0 text-center text-[13px] font-semibold leading-none text-zinc-500", "&" }
                            span { class: "truncate text-[12px]", "Разработка" }
                        }
                    }
                    button { r#type: "button", "data-active": if active_room == "Тест звука" { "true" } else { "false" }, class: "flex w-full items-center justify-between rounded-lg border border-transparent px-2.5 py-2 text-left text-zinc-400 transition-[background,border-color,color,transform,opacity] duration-150 hover:border-zinc-800 hover:bg-zinc-900 hover:text-zinc-100 data-[active=true]:border-accent/25 data-[active=true]:bg-accent/10 data-[active=true]:text-zinc-100",
                        onclick: move |_| on_select_room.call(ActiveRoom { kind: "mixed-chat", name: "Тест звука" }),
                        span { class: "flex min-w-0 items-center gap-2",
                            span { class: "w-3.5 shrink-0 text-center text-[13px] font-semibold leading-none text-zinc-500", "&" }
                            span { class: "truncate text-[12px]", "Тест звука" }
                        }
                    }
                    button { r#type: "button", "data-active": if active_room == "общий" { "true" } else { "false" }, class: "flex w-full items-center justify-between rounded-lg border border-transparent px-2.5 py-2 text-left text-zinc-400 transition-[background,border-color,color,transform,opacity] duration-150 hover:border-zinc-800 hover:bg-zinc-900 hover:text-zinc-100 data-[active=true]:border-accent/25 data-[active=true]:bg-accent/10 data-[active=true]:text-zinc-100",
                        onclick: move |_| on_select_room.call(ActiveRoom { kind: "text", name: "общий" }),
                        span { class: "flex min-w-0 items-center gap-2",
                            span { class: "w-3.5 shrink-0 text-center text-zinc-600", "#" }
                            span { class: "truncate text-[12px]", "общий" }
                        }
                    }
                    button { r#type: "button", "data-active": if active_room == "релизы" { "true" } else { "false" }, class: "flex w-full items-center gap-2 rounded-lg border border-transparent px-2.5 py-2 text-left text-zinc-400 transition-[background,border-color,color,transform,opacity] duration-150 hover:border-zinc-800 hover:bg-zinc-900 hover:text-zinc-100 data-[active=true]:border-accent/25 data-[active=true]:bg-accent/10 data-[active=true]:text-zinc-100",
                        onclick: move |_| on_select_room.call(ActiveRoom { kind: "text", name: "релизы" }),
                        span { class: "w-3.5 shrink-0 text-center text-zinc-600", "#" }
                        span { class: "truncate text-[12px]", "релизы" }
                    }
                    button { r#type: "button", "data-active": if active_room == "баги" { "true" } else { "false" }, class: "flex w-full items-center gap-2 rounded-lg border border-transparent px-2.5 py-2 text-left text-zinc-400 transition-[background,border-color,color,transform,opacity] duration-150 hover:border-zinc-800 hover:bg-zinc-900 hover:text-zinc-100 data-[active=true]:border-accent/25 data-[active=true]:bg-accent/10 data-[active=true]:text-zinc-100",
                        onclick: move |_| on_select_room.call(ActiveRoom { kind: "text", name: "баги" }),
                        span { class: "w-3.5 shrink-0 text-center text-zinc-600", "#" }
                        span { class: "truncate text-[12px]", "баги" }
                    }
                    button { r#type: "button", class: "flex w-full items-center justify-between rounded-lg border border-transparent px-2.5 py-2 text-left text-zinc-400 transition-[background,border-color,color,transform,opacity] duration-150 hover:border-zinc-800 hover:bg-zinc-900 hover:text-zinc-100",
                        span { class: "flex min-w-0 items-center gap-2",
                            span { class: "w-3.5 shrink-0 text-center text-[13px] font-semibold leading-none text-zinc-500", "~" }
                            span { class: "truncate text-[12px]", "AFK" }
                        }
                    }
                }
            }

            div { class: "border-t border-zinc-800/80 p-3",
                div { class: "relative mb-2 rounded-[20px] border border-zinc-800 bg-zinc-900/80 p-2.5",
                    div { class: "mb-2 flex items-start gap-2",
                        button { r#type: "button", class: "group relative flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-emerald-500/20 bg-emerald-500/10 text-emerald-300 hover:border-emerald-400/35 hover:bg-emerald-500/15", "aria-label": "Качество связи",
                            span { class: "pointer-events-none absolute bottom-[calc(100%+10px)] left-0 z-[90] w-max min-w-[190px] translate-y-1 rounded-xl border border-zinc-800 bg-zinc-950/95 p-3 text-left opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100",
                                span { class: "block text-[12px] font-medium text-zinc-100", "Качество связи: хорошее" }
                                span { class: "mt-1 block text-[11px] text-zinc-500", "Пинг: 24 мс · Потери: 0.3%" }
                                span { class: "mt-2 block text-[10px] text-zinc-600", "Нажмите для подробностей" }
                            }
                            svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 18.5v-3.25" }
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8.5 18.5h7" }
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.25 13.75a4 4 0 0 1 5.5 0" }
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6.5 11a8 8 0 0 1 11 0" }
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3.75 8.25a12 12 0 0 1 16.5 0" }
                                circle { cx: "12", cy: "18.5", r: "1.15", fill: "currentColor", stroke: "none" }
                            }
                        }
                        div { class: "min-w-0 flex-1",
                            div { class: "truncate text-[11px] font-medium text-zinc-100", "{server_name}" }
                            div { class: "mt-0.5 flex min-w-0 items-center gap-1.5 text-[11px] text-zinc-500",
                                svg { class: "h-3.5 w-3.5 shrink-0 text-zinc-600", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11a7 7 0 0 1-14 0m7 8v3m-4 0h8m-4-18a3 3 0 0 0-3 3v4a3 3 0 1 0 6 0V7a3 3 0 0 0-3-3Z" }
                                }
                                span { class: "truncate", "Лобби" }
                            }
                        }
                    }
                    div { class: "grid grid-cols-4 gap-2",
                        button { r#type: "button", class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/70 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900", "aria-label": "Выключить микрофон",
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11a7 7 0 0 1-14 0m7 8v3m-4 0h8m-4-18a3 3 0 0 0-3 3v4a3 3 0 1 0 6 0V7a3 3 0 0 0-3-3Z" }
                            }
                        }
                        button { r#type: "button", class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/70 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900", "aria-label": "Отключить звук",
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19.114 5.636a9 9 0 0 1 0 12.728M16.463 8.288a5.25 5.25 0 0 1 0 7.424M6.75 8.25 10.5 4.5v15l-3.75-3.75H3.75A1.5 1.5 0 0 1 2.25 14.25v-4.5A1.5 1.5 0 0 1 3.75 8.25h3Z" }
                            }
                        }
                        button { r#type: "button", class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/70 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900", "aria-label": "Настройки голосовой сессии",
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10.5 6h9.75M10.5 6a1.5 1.5 0 1 1-3 0m3 0a1.5 1.5 0 1 0-3 0M3.75 6H7.5m9 6h3.75m-3.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-9.75 0h9.75m-3 6h9.75m-9.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-3.75 0H7.5" }
                            }
                        }
                        button { r#type: "button", class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-red-500/35 hover:bg-red-500/10 hover:text-red-200", "aria-label": "Выйти из голосового чата",
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9" }
                            }
                        }
                    }
                }
                div { class: "flex items-center gap-3 rounded-[20px] border border-zinc-800 bg-zinc-900/80 p-2.5",
                    div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-accent text-[12px] font-bold text-white", "Ч" }
                    div { class: "min-w-0 flex-1",
                        div { class: "truncate text-[12px] font-medium text-zinc-100", "chingiz" }
                        div { class: "truncate text-[11px] text-zinc-500", "в голосе · микрофон включен" }
                    }
                    button { r#type: "button", class: "rounded-lg p-2 text-zinc-500 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-zinc-800 hover:text-zinc-200", "aria-label": "Настройки",
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.075.04.149.083.22.127.324.2.72.226 1.075.091l1.217-.462a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.82c-.293.24-.438.613-.431.992a7.723 7.723 0 0 1 0 .255c-.007.379.138.752.431.992l1.003.82c.433.354.54.972.26 1.431l-1.296 2.247a1.125 1.125 0 0 1-1.37.49l-1.217-.462c-.355-.135-.751-.109-1.076.091a6.72 6.72 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.542-.56.94-1.11.94h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.063-.374-.313-.686-.645-.87a6.52 6.52 0 0 1-.22-.127c-.324-.2-.72-.226-1.075-.091l-1.217.462a1.125 1.125 0 0 1-1.37-.49l-1.296-2.247a1.125 1.125 0 0 1 .26-1.431l1.003-.82c.293-.24.438-.613.431-.992a6.932 6.932 0 0 1 0-.255c.007-.379-.138-.752-.431-.992l-1.003-.82a1.125 1.125 0 0 1-.26-1.431l1.296-2.247a1.125 1.125 0 0 1 1.37-.49l1.217.462c.355-.135.751-.109 1.076-.091.071-.044.145-.087.22-.128.331-.183.581-.495.644-.869l.213-1.281Z" }
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" }
                        }
                    }
                }
            }
        }
    }
}

//! Server rail component for the authenticated app.

use cheenhub_contracts::rest::ServerSummary;
use dioxus::prelude::*;

use crate::features::landing::components::logo_icon::LogoIcon;

use super::native_client_download::NativeClientDownload;
use super::server_rail_button::ServerRailButton;

/// Renders the server switcher rail.
#[component]
pub(crate) fn ServerRail(
    servers: Vec<ServerSummary>,
    active_server_id: Option<String>,
    social_active: bool,
    host_settings_active: bool,
    host_access_loading: bool,
    host_access_error: Option<String>,
    is_host_owner: bool,
    is_loading: bool,
    status: String,
    on_select_server: EventHandler<String>,
    on_open_social: EventHandler<()>,
    on_open_host_settings: EventHandler<()>,
    on_retry_host_access: EventHandler<()>,
    on_add_server: EventHandler<()>,
) -> Element {
    let mut show_empty_server_hint = use_signal(|| true);
    let list_class = if !is_loading && servers.is_empty() {
        "space-y-2 overflow-visible pb-3"
    } else {
        "space-y-2 overflow-y-auto pb-3"
    };
    let logo_class = if social_active {
        "mb-4 flex h-12 w-12 items-center justify-center rounded-2xl border border-blue-400/35 bg-blue-500/15 text-blue-100 shadow-[0_0_0_1px_rgba(59,130,246,0.12)] transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px"
    } else {
        "mb-4 flex h-12 w-12 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-100 text-zinc-950 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-blue-400/30 hover:bg-blue-50"
    };

    rsx! {
        aside { class: "server-rail relative z-[90] flex w-[76px] shrink-0 flex-col overflow-visible border-r border-zinc-800/80 bg-zinc-950/85 p-3 backdrop-blur-xl",
            button {
                r#type: "button",
                class: logo_class,
                "aria-label": "Друзья и личные сообщения",
                onclick: move |_| on_open_social.call(()),
                LogoIcon { class_name: "h-7 w-7" }
            }
            div { class: list_class,
                if is_loading {
                    div { class: "flex h-12 w-12 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-900/60 text-[11px] font-semibold text-zinc-500",
                        "..."
                    }
                } else if servers.is_empty() {
                    div { class: "relative",
                        button {
                            r#type: "button",
                            class: "flex h-12 w-12 items-center justify-center rounded-2xl border border-dashed border-accent/35 bg-accent/10 text-accent transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-accent/55 hover:bg-accent/15",
                            "aria-label": "Создать первый сервер",
                            onclick: move |_| on_add_server.call(()),
                            svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 5v14m-7-7h14" }
                            }
                        }
                        if show_empty_server_hint() {
                            div { class: "pointer-events-none absolute left-[calc(100%+12px)] top-0 z-[80] w-[230px] rounded-xl border border-zinc-800 bg-zinc-950/95 py-2 pl-3 pr-10 text-left shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl",
                                p { class: "text-pretty text-[12px] font-semibold text-zinc-100", "Здесь появятся твои серверы" }
                                p { class: "mt-1 text-pretty text-[11px] leading-4 text-zinc-400", "Создай первый сервер для друзей или команды." }
                                button {
                                    r#type: "button",
                                    class: "pointer-events-auto absolute right-0 top-0 flex size-10 items-center justify-center rounded-xl text-zinc-500 transition-[background-color,color,scale] duration-150 ease-out hover:bg-white/5 hover:text-zinc-200 active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/70",
                                    "aria-label": "Закрыть подсказку о серверах",
                                    onclick: move |_| {
                                        info!("dismissed empty server rail hint");
                                        show_empty_server_hint.set(false);
                                    },
                                    svg { class: "size-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 6l12 12M18 6L6 18" }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    for server in servers {
                        ServerRailButton {
                            server: server.clone(),
                            active: active_server_id.as_deref() == Some(server.id.as_str()),
                            on_select_server
                        }
                    }
                }
            }
            if !status.is_empty() {
                p { class: "mb-2 rounded-xl border border-red-500/15 bg-red-500/10 px-2 py-1.5 text-center text-[10px] leading-4 text-red-200", "aria-label": "{status}",
                    "!"
                }
            }
            div { class: "mt-auto border-t border-zinc-800/80 pt-3",
                button {
                    r#type: "button",
                    class: "flex h-12 w-12 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-900/80 text-zinc-400 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-accent/30 hover:bg-accent/10 hover:text-zinc-200",
                    "aria-label": "Добавить сервер",
                    onclick: move |_| on_add_server.call(()),
                    svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 5v14m-7-7h14" }
                    }
                }
                if host_access_loading {
                    div {
                        class: "mt-2 flex h-12 w-12 animate-pulse items-center justify-center rounded-2xl bg-zinc-900/70 shadow-[0_0_0_1px_rgba(255,255,255,0.06)]",
                        "aria-label": "Проверяем доступ к настройкам хоста"
                    }
                } else if let Some(error) = host_access_error {
                    button {
                        r#type: "button",
                        class: "mt-2 flex h-12 w-12 items-center justify-center rounded-2xl bg-red-500/10 text-sm font-bold text-red-200 shadow-[0_0_0_1px_rgba(248,113,113,0.2)] transition-[background-color,color,scale,box-shadow] duration-150 ease-out hover:bg-red-500/15 hover:shadow-[0_0_0_1px_rgba(248,113,113,0.32)] active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-400/70",
                        "aria-label": "Не удалось проверить доступ к настройкам хоста. Нажми, чтобы повторить",
                        title: "{error}",
                        onclick: move |_| on_retry_host_access.call(()),
                        "!"
                    }
                } else if is_host_owner {
                    button {
                        r#type: "button",
                        class: if host_settings_active {
                            "relative mt-2 flex h-12 w-12 items-center justify-center rounded-2xl bg-blue-500 text-white shadow-[0_0_0_1px_rgba(96,165,250,0.35),0_10px_28px_rgba(59,130,246,0.2)] transition-[background-color,color,scale,box-shadow] duration-150 ease-out active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/80"
                        } else {
                            "mt-2 flex h-12 w-12 items-center justify-center rounded-2xl bg-zinc-900/80 text-zinc-400 shadow-[0_0_0_1px_rgba(255,255,255,0.08)] transition-[background-color,color,scale,box-shadow] duration-150 ease-out hover:bg-blue-500/10 hover:text-blue-100 hover:shadow-[0_0_0_1px_rgba(96,165,250,0.25)] active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400/70"
                        },
                        "aria-label": "Настройки хоста",
                        title: "Настройки хоста",
                        onclick: move |_| on_open_host_settings.call(()),
                        if host_settings_active {
                            span { class: "absolute -left-3 h-7 w-1 rounded-r-full bg-blue-400" }
                        }
                        svg { class: "size-5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z" }
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.5V21h-4v-.1a1.7 1.7 0 0 0-1-1.5 1.7 1.7 0 0 0-1.9.3l-.1.1L4.2 17l.1-.1a1.7 1.7 0 0 0 .3-1.9 1.7 1.7 0 0 0-1.5-1H3v-4h.1a1.7 1.7 0 0 0 1.5-1 1.7 1.7 0 0 0-.3-1.9L4.2 7 7 4.2l.1.1a1.7 1.7 0 0 0 1.9.3 1.7 1.7 0 0 0 1-1.5V3h4v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.5 1h.1v4h-.1a1.7 1.7 0 0 0-1.5 1Z" }
                        }
                    }
                }
                NativeClientDownload {}
            }
        }
    }
}

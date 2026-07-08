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
    is_loading: bool,
    status: String,
    on_select_server: EventHandler<String>,
    on_open_social: EventHandler<()>,
    on_add_server: EventHandler<()>,
) -> Element {
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
                        div { class: "pointer-events-none absolute left-[calc(100%+12px)] top-0 z-[80] w-[230px] rounded-xl border border-zinc-800 bg-zinc-950/95 px-3 py-2 text-left shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl",
                            p { class: "text-[12px] font-semibold text-zinc-100", "Здесь появятся твои серверы" }
                            p { class: "mt-1 text-[11px] leading-4 text-zinc-400", "Создай первый сервер для друзей или команды." }
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
                NativeClientDownload {}
            }
        }
    }
}

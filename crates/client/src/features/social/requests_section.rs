//! Секция заявок в друзья на social-экране.

use cheenhub_contracts::rest::FriendRequestSummary;
use dioxus::prelude::*;

/// Рендерит сворачиваемый список входящих и исходящих заявок в друзья.
#[component]
pub(super) fn FriendRequestsSection(
    incoming: Vec<FriendRequestSummary>,
    outgoing: Vec<FriendRequestSummary>,
    collapsed: bool,
    requests_count: usize,
    on_toggle: EventHandler<()>,
    on_accept: EventHandler<String>,
    on_decline: EventHandler<String>,
    on_cancel: EventHandler<String>,
) -> Element {
    rsx! {
        section {
            button {
                r#type: "button",
                class: "flex w-full items-center justify-between rounded-lg px-1 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-zinc-500 transition hover:bg-zinc-900/70 hover:text-zinc-300",
                "aria-expanded": if collapsed { "false" } else { "true" },
                onclick: move |_| on_toggle.call(()),
                span { "Заявки" }
                span { class: "flex items-center gap-2",
                    span { class: "rounded-full border border-zinc-800 bg-zinc-900 px-2 py-0.5 text-[10px] text-zinc-400", "{requests_count}" }
                    svg {
                        class: if collapsed { "h-3.5 w-3.5 transition-transform" } else { "h-3.5 w-3.5 rotate-180 transition-transform" },
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        view_box: "0 0 24 24",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "m6 9 6 6 6-6" }
                    }
                }
            }
            if !collapsed {
                if incoming.is_empty() && outgoing.is_empty() {
                    p { class: "mt-2 rounded-lg border border-zinc-800 bg-zinc-900/50 px-3 py-3 text-[12px] leading-5 text-zinc-500",
                        "Новых заявок пока нет."
                    }
                } else {
                    div { class: "mt-2 space-y-2",
                        for (request, accept_request_id, decline_request_id) in incoming.into_iter().map(|request| {
                            let accept_request_id = request.id.clone();
                            let decline_request_id = request.id.clone();
                            (request, accept_request_id, decline_request_id)
                        }) {
                            div { key: "{request.id}", class: "rounded-lg border border-zinc-800 bg-zinc-900/50 p-2",
                                p { class: "truncate text-[13px] font-medium text-zinc-100", "{request.sender_nickname}" }
                                div { class: "mt-2 flex gap-2",
                                    button {
                                        class: "h-8 flex-1 rounded-md bg-emerald-500/15 text-[12px] font-medium text-emerald-200 hover:bg-emerald-500/25",
                                        onclick: move |_| on_accept.call(accept_request_id.clone()),
                                        "Принять"
                                    }
                                    button {
                                        class: "h-8 flex-1 rounded-md bg-zinc-800 text-[12px] font-medium text-zinc-300 hover:bg-zinc-700",
                                        onclick: move |_| on_decline.call(decline_request_id.clone()),
                                        "Отклонить"
                                    }
                                }
                            }
                        }
                        for request in outgoing {
                            div { key: "{request.id}", class: "flex items-center justify-between gap-2 rounded-lg border border-zinc-800 bg-zinc-900/50 p-2",
                                div { class: "min-w-0",
                                    p { class: "truncate text-[13px] font-medium text-zinc-100", "{request.recipient_nickname}" }
                                    p { class: "text-[11px] text-zinc-500", "Ожидает ответа" }
                                }
                                button {
                                    class: "h-8 rounded-md bg-zinc-800 px-3 text-[12px] font-medium text-zinc-300 hover:bg-zinc-700",
                                    onclick: move |_| on_cancel.call(request.id.clone()),
                                    "Отменить"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

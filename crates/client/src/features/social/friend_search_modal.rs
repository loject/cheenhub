//! Модальное окно поиска новых друзей.

use cheenhub_contracts::rest::UserSearchResult;
use dioxus::prelude::*;

use crate::features::app::components::avatar::UserAvatar;

use super::api;
use super::presentation::relation_label;

/// Рендерит поиск пользователей и отправку заявок в друзья.
#[component]
pub(super) fn FriendSearchModal(
    on_close: EventHandler<()>,
    on_changed: EventHandler<()>,
) -> Element {
    let mut query = use_signal(String::new);
    let mut results = use_signal(Vec::<UserSearchResult>::new);
    let mut status = use_signal(String::new);
    let mut is_searching = use_signal(|| false);

    let run_search = use_callback(move |_| {
        let request_query = query().trim().to_owned();
        if request_query.len() < 2 {
            results.set(Vec::new());
            status.set("Введите минимум два символа.".to_owned());
            return;
        }

        is_searching.set(true);
        status.set(String::new());
        spawn(async move {
            match api::search_users(&request_query).await {
                Ok(users) => {
                    results.set(users);
                    status.set(String::new());
                }
                Err(error) => status.set(error),
            }
            is_searching.set(false);
        });
    });

    rsx! {
        div {
            class: "fixed inset-0 z-[950] flex items-center justify-center bg-black/65 px-4 py-6 backdrop-blur-sm",
            onclick: move |_| on_close.call(()),
            section {
                class: "flex max-h-[min(720px,calc(100vh-48px))] w-full max-w-lg flex-col overflow-hidden rounded-lg border border-zinc-800 bg-zinc-950 text-zinc-100 shadow-[0_24px_80px_rgba(0,0,0,.55)]",
                onclick: move |event| event.stop_propagation(),
                div { class: "flex h-14 shrink-0 items-center justify-between border-b border-zinc-800 px-4",
                    div {
                        h2 { class: "text-[15px] font-semibold text-zinc-50", "Найти друзей" }
                        p { class: "text-[12px] text-zinc-500", "Поиск по никнейму CheenHub" }
                    }
                    button {
                        r#type: "button",
                        class: "flex h-9 w-9 items-center justify-center rounded-md text-zinc-400 transition hover:bg-zinc-900 hover:text-zinc-100",
                        "aria-label": "Закрыть поиск",
                        onclick: move |_| on_close.call(()),
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18 18 6M6 6l12 12" }
                        }
                    }
                }

                div { class: "shrink-0 border-b border-zinc-800 p-4",
                    div { class: "flex items-center gap-2 rounded-lg border border-zinc-800 bg-zinc-900/70 px-3 py-2",
                        input {
                            class: "min-w-0 flex-1 bg-transparent text-[13px] text-zinc-100 outline-none placeholder:text-zinc-600",
                            value: "{query()}",
                            placeholder: "Никнейм пользователя",
                            oninput: move |event| query.set(event.value()),
                            onkeydown: move |event| {
                                if event.key() == Key::Enter {
                                    event.prevent_default();
                                    run_search.call(());
                                }
                            },
                        }
                        button {
                            r#type: "button",
                            disabled: is_searching(),
                            class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-blue-500 text-white transition hover:bg-blue-400 disabled:opacity-50",
                            "aria-label": "Найти пользователя",
                            onclick: move |_| run_search.call(()),
                            if is_searching() {
                                span { class: "h-4 w-4 animate-spin rounded-full border-2 border-blue-200/40 border-t-white" }
                            } else {
                                svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "m21 21-4.3-4.3M10.5 18a7.5 7.5 0 1 1 0-15 7.5 7.5 0 0 1 0 15Z" }
                                }
                            }
                        }
                    }
                    if !status().is_empty() {
                        p { class: "mt-2 text-[12px] leading-5 text-zinc-500", "{status()}" }
                    }
                }

                div { class: "min-h-0 flex-1 overflow-y-auto p-3",
                    if is_searching() && results().is_empty() {
                        div { class: "space-y-2",
                            div { class: "h-12 animate-pulse rounded-lg bg-zinc-900/80" }
                            div { class: "h-12 animate-pulse rounded-lg bg-zinc-900/50" }
                        }
                    } else if results().is_empty() {
                        p { class: "rounded-lg border border-zinc-800 bg-zinc-900/50 px-3 py-4 text-center text-[12px] leading-5 text-zinc-500",
                            "Введите никнейм и нажмите поиск."
                        }
                    } else {
                        div { class: "space-y-1",
                            for user in results() {
                                div { key: "{user.id}", class: "flex items-center gap-2 rounded-lg px-2 py-2 hover:bg-zinc-900/70",
                                    UserAvatar {
                                        nickname: user.nickname.clone(),
                                        avatar_url: user.avatar_url.clone(),
                                        class: "h-10 w-10 shrink-0 rounded-lg border border-zinc-800 bg-zinc-900 text-[12px] font-bold text-zinc-100".to_owned(),
                                        avatar_seed: Some(user.id.clone()),
                                    }
                                    div { class: "min-w-0 flex-1",
                                        p { class: "truncate text-[13px] font-medium text-zinc-100", "{user.nickname}" }
                                        p { class: "text-[11px] text-zinc-500", "{relation_label(user.relation)}" }
                                    }
                                    if user.relation.is_none() {
                                        button {
                                            r#type: "button",
                                            class: "flex h-9 w-9 items-center justify-center rounded-md border border-blue-400/30 bg-blue-500/10 text-blue-200 transition hover:bg-blue-500/20",
                                            "aria-label": "Отправить заявку",
                                            onclick: move |_| {
                                                let user_id = user.id.clone();
                                                spawn(async move {
                                                    match api::send_friend_request(user_id).await {
                                                        Ok(_) => {
                                                            run_search.call(());
                                                            on_changed.call(());
                                                        }
                                                        Err(error) => status.set(error),
                                                    }
                                                });
                                            },
                                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 5v14m-7-7h14" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

//! Server invite acceptance page.

use cheenhub_contracts::rest::ServerInviteInfoResponse;
use dioxus::prelude::*;

use crate::Route;
use crate::features::app::api;

/// Renders a server invite acceptance page.
#[component]
pub(crate) fn InvitePage(code: String) -> Element {
    let navigator = use_navigator();
    let mut is_joined = use_signal(|| false);
    let mut load_state = use_signal(|| InviteLoadState::Loading);
    let mut requested_key = use_signal(|| None::<(String, u64)>);
    let mut retry_generation = use_signal(|| 0_u64);

    use_effect(move || {
        let retry = retry_generation();
        let request_code = code.clone();
        let next_key = (request_code.clone(), retry);

        if requested_key() == Some(next_key.clone()) {
            return;
        }

        requested_key.set(Some(next_key));
        load_state.set(InviteLoadState::Loading);

        spawn(async move {
            match api::load_server_invite(request_code).await {
                Ok(invite) => load_state.set(InviteLoadState::Loaded(invite)),
                Err(error) => load_state.set(InviteLoadState::Failed(error)),
            }
        });
    });

    match load_state() {
        InviteLoadState::Loading => invite_loader(),
        InviteLoadState::Failed(error) => {
            let show_login = error == "Войди, чтобы продолжить.";

            rsx! {
                main { class: "min-h-screen bg-zinc-950 px-4 py-6 text-zinc-100 sm:px-6 lg:px-8",
                    div { class: "mx-auto grid min-h-[calc(100vh-3rem)] w-full max-w-3xl place-items-center",
                        section { class: "w-full rounded-3xl border border-zinc-800 bg-zinc-900/70 p-6 shadow-2xl shadow-black/30 sm:p-8",
                            div { class: "flex h-14 w-14 items-center justify-center rounded-2xl border border-rose-500/20 bg-rose-500/10 text-rose-200",
                                svg { class: "h-6 w-6", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 9v4m0 4h.01M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0Z" }
                                }
                            }
                            h1 { class: "mt-5 text-3xl font-semibold text-white", "Не удалось открыть приглашение" }
                            p { class: "mt-3 max-w-xl text-[15px] leading-7 text-zinc-400", "{error}" }
                            div { class: "mt-6 flex flex-col gap-3 sm:flex-row",
                                button {
                                    r#type: "button",
                                    class: "flex h-12 w-full items-center justify-center rounded-xl bg-accent px-4 text-[14px] font-semibold text-white transition-[background,transform] duration-150 hover:-translate-y-px hover:bg-blue-400 sm:w-auto",
                                    onclick: move |_| retry_generation.set(retry_generation() + 1),
                                    "Повторить"
                                }
                                if show_login {
                                    button {
                                        r#type: "button",
                                        class: "flex h-12 w-full items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 px-4 text-[14px] font-medium text-zinc-300 transition-[background,border-color,color,transform] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900 hover:text-white sm:w-auto",
                                        onclick: move |_| {
                                            navigator.push(Route::Login {});
                                        },
                                        "Войти"
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "flex h-12 w-full items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 px-4 text-[14px] font-medium text-zinc-300 transition-[background,border-color,color,transform] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900 hover:text-white sm:w-auto",
                                    onclick: move |_| {
                                        navigator.push(Route::AppHome {});
                                    },
                                    "Открыть CheenHub"
                                }
                            }
                        }
                    }
                }
            }
        }
        InviteLoadState::Loaded(invite) => {
            let server_name = invite.server.name.clone();
            let server_initials = initials(&server_name);
            let invite_code = invite.invite.code.clone();
            let limit_text = usage_limit_text(invite.invite.max_uses);
            let expiration_text = expiration_text(invite.invite.expires_at.as_deref());
            let is_current_user_server_member = invite.server.is_owner;
            let headline = if is_current_user_server_member {
                format!("Ты уже на сервере {server_name}")
            } else {
                format!("Вступить на сервер {server_name}")
            };
            let description = if is_current_user_server_member {
                "Это приглашение ведет на сервер, который уже доступен в твоем CheenHub."
            } else {
                "Тебя пригласили в рабочее пространство для общения команды. После принятия сервер появится в списке доступных серверов."
            };
            let ownership_text = if is_current_user_server_member {
                "Ты уже участник этого сервера"
            } else {
                "Доступно по приглашению"
            };
            let action_label = if is_current_user_server_member {
                "Открыть сервер"
            } else if is_joined() {
                "Приглашение принято"
            } else {
                "Принять приглашение"
            };
            let action_class = if is_current_user_server_member {
                "flex h-12 w-full items-center justify-center rounded-xl bg-emerald-500 px-4 text-[14px] font-semibold text-emerald-950 transition-[background,transform] duration-150 hover:-translate-y-px hover:bg-emerald-400 sm:w-auto sm:min-w-48"
            } else if is_joined() {
                "flex h-12 w-full items-center justify-center rounded-xl bg-emerald-500 px-4 text-[14px] font-semibold text-emerald-950 transition sm:w-auto sm:min-w-48"
            } else {
                "flex h-12 w-full items-center justify-center rounded-xl bg-accent px-4 text-[14px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_12px_34px_rgba(59,130,246,0.22)] transition-[background,transform,opacity] duration-150 hover:-translate-y-px hover:bg-blue-400 sm:w-auto sm:min-w-48"
            };

            rsx! {
                main { class: "min-h-screen bg-zinc-950 px-4 py-6 text-zinc-100 sm:px-6 lg:px-8",
                    div { class: "mx-auto flex min-h-[calc(100vh-3rem)] w-full max-w-5xl items-center",
                        section { class: "grid w-full gap-6 lg:grid-cols-[1fr_360px] lg:items-center",
                            div { class: "space-y-8",
                                div { class: "inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/70 px-3 py-1.5 text-[12px] font-medium text-zinc-300",
                                    span { class: "h-2 w-2 rounded-full bg-emerald-400" }
                                    "Приглашение CheenHub"
                                }

                                div { class: "max-w-2xl space-y-4",
                                    h1 { class: "text-4xl font-semibold leading-tight text-white sm:text-5xl",
                                        "{headline}"
                                    }
                                    p { class: "max-w-xl text-[15px] leading-7 text-zinc-400",
                                        "{description}"
                                    }
                                }

                                div { class: "flex flex-col gap-3 sm:flex-row",
                                    button {
                                        r#type: "button",
                                        class: "{action_class}",
                                        disabled: !is_current_user_server_member && is_joined(),
                                        onclick: move |_| {
                                            if is_current_user_server_member {
                                                navigator.push(Route::AppHome {});
                                            } else {
                                                is_joined.set(true);
                                            }
                                        },
                                        "{action_label}"
                                    }
                                    button {
                                        r#type: "button",
                                        class: "flex h-12 w-full items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 px-4 text-[14px] font-medium text-zinc-300 transition-[background,border-color,color,transform] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900 hover:text-white sm:w-auto",
                                        onclick: move |_| {
                                            navigator.push(Route::AppHome {});
                                        },
                                        "Открыть CheenHub"
                                    }
                                }

                                if is_current_user_server_member {
                                    div { class: "max-w-xl rounded-2xl border border-emerald-500/20 bg-emerald-500/10 px-4 py-3 text-[13px] leading-6 text-emerald-100",
                                        "Сервер уже доступен в твоем списке. Можно открыть CheenHub и перейти к нему."
                                    }
                                }

                                if is_joined() {
                                    div { class: "max-w-xl rounded-2xl border border-emerald-500/20 bg-emerald-500/10 px-4 py-3 text-[13px] leading-6 text-emerald-100",
                                        "Приглашение отмечено как принято. Можешь открыть CheenHub и продолжить работу."
                                    }
                                }
                            }

                            aside { class: "rounded-3xl border border-zinc-800 bg-zinc-900/70 p-5 shadow-2xl shadow-black/30",
                                div { class: "flex items-start gap-4",
                                    div { class: "flex h-16 w-16 shrink-0 items-center justify-center rounded-2xl bg-blue-500 text-2xl font-bold text-white",
                                        "{server_initials}"
                                    }
                                    div { class: "min-w-0 pt-1",
                                        p { class: "truncate text-lg font-semibold text-white", "{server_name}" }
                                        p { class: "mt-1 text-[13px] text-zinc-400", "{ownership_text}" }
                                    }
                                }

                                div { class: "mt-6 space-y-3",
                                    div { class: "rounded-2xl bg-zinc-950/70 p-3",
                                        p { class: "text-[11px] font-medium uppercase text-zinc-500", "Использования" }
                                        p { class: "mt-1 text-[14px] font-semibold text-zinc-100", "{limit_text}" }
                                    }
                                    div { class: "rounded-2xl bg-zinc-950/70 p-3",
                                        p { class: "text-[11px] font-medium uppercase text-zinc-500", "Срок действия" }
                                        p { class: "mt-1 text-[14px] font-semibold text-zinc-100", "{expiration_text}" }
                                    }
                                }

                                div { class: "mt-5 rounded-2xl border border-zinc-800 bg-zinc-950/50 p-3",
                                    p { class: "text-[12px] font-medium text-zinc-300", "Код приглашения" }
                                    p { class: "mt-2 break-all font-mono text-[12px] leading-5 text-zinc-500", "{invite_code}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
enum InviteLoadState {
    Loading,
    Loaded(ServerInviteInfoResponse),
    Failed(String),
}

fn invite_loader() -> Element {
    rsx! {
        main { class: "min-h-screen bg-zinc-950 px-4 py-6 text-zinc-100 sm:px-6 lg:px-8",
            div { class: "mx-auto flex min-h-[calc(100vh-3rem)] w-full max-w-5xl items-center",
                section { class: "grid w-full gap-6 lg:grid-cols-[1fr_360px] lg:items-center",
                    div { class: "space-y-8",
                        div { class: "h-8 w-44 animate-pulse rounded-full bg-zinc-900" }
                        div { class: "max-w-2xl space-y-4",
                            div { class: "h-12 w-full max-w-xl animate-pulse rounded-2xl bg-zinc-900" }
                            div { class: "h-12 w-4/5 max-w-lg animate-pulse rounded-2xl bg-zinc-900" }
                            div { class: "h-5 w-full max-w-xl animate-pulse rounded-full bg-zinc-900" }
                            div { class: "h-5 w-3/4 max-w-md animate-pulse rounded-full bg-zinc-900" }
                        }
                        div { class: "flex flex-col gap-3 sm:flex-row",
                            div { class: "h-12 w-full animate-pulse rounded-xl bg-zinc-900 sm:w-48" }
                            div { class: "h-12 w-full animate-pulse rounded-xl bg-zinc-900 sm:w-44" }
                        }
                    }
                    aside { class: "rounded-3xl border border-zinc-800 bg-zinc-900/70 p-5 shadow-2xl shadow-black/30",
                        div { class: "flex items-start gap-4",
                            div { class: "h-16 w-16 shrink-0 animate-pulse rounded-2xl bg-zinc-800" }
                            div { class: "min-w-0 flex-1 pt-1",
                                div { class: "h-5 w-3/4 animate-pulse rounded-full bg-zinc-800" }
                                div { class: "mt-3 h-4 w-1/2 animate-pulse rounded-full bg-zinc-800" }
                            }
                        }
                        div { class: "mt-6 space-y-3",
                            div { class: "h-16 animate-pulse rounded-2xl bg-zinc-950/70" }
                            div { class: "h-16 animate-pulse rounded-2xl bg-zinc-950/70" }
                        }
                        div { class: "mt-5 h-20 animate-pulse rounded-2xl bg-zinc-950/50" }
                    }
                }
            }
        }
    }
}

fn initials(name: &str) -> String {
    let mut chars = name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .flat_map(char::to_uppercase);
    let first = chars.next().unwrap_or('C');
    let second = chars.next().unwrap_or('H');

    format!("{first}{second}")
}

fn usage_limit_text(max_uses: Option<u32>) -> String {
    max_uses
        .map(|uses| format!("До {uses} входов"))
        .unwrap_or_else(|| "Без ограничения".to_owned())
}

fn expiration_text(expires_at: Option<&str>) -> String {
    expires_at
        .map(|value| format!("До {value}"))
        .unwrap_or_else(|| "Без срока".to_owned())
}

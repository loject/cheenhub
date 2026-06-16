//! User security settings section.

use cheenhub_contracts::rest::{ActiveSession, SessionDeviceKind};
use dioxus::prelude::*;

use crate::Route;
use crate::features::auth::sessions_api;
use crate::features::toast::ToastHandle;

use super::password_section::PasswordSettingsSection;

/// Renders account security controls.
#[component]
pub(crate) fn SecuritySettingsSection() -> Element {
    let navigator = use_navigator();
    let toast = use_context::<ToastHandle>();
    let mut sessions_resource = use_resource(sessions_api::active_sessions);
    let sessions_result = sessions_resource.read().clone();
    let mut pending_session = use_signal(|| None::<String>);
    let mut pending_all = use_signal(|| false);
    let is_loading = sessions_result.is_none();
    let any_action_pending = pending_all() || pending_session().is_some();
    let loaded_session_count = loaded_session_count(&sessions_result);

    let on_revoke_session = EventHandler::new(move |session_id: String| {
        if pending_all() || pending_session().is_some() {
            return;
        }

        pending_session.set(Some(session_id.clone()));
        info!(session_id = %session_id, "revoking auth session from security settings");
        spawn(async move {
            match sessions_api::revoke_session(&session_id).await {
                Ok(()) => {
                    info!(session_id = %session_id, "auth session revoked from security settings");
                    toast.success("Сеанс завершен.");
                    pending_session.set(None);
                    sessions_resource.clear();
                    sessions_resource.restart();
                }
                Err(error) => {
                    warn!(
                        %error,
                        session_id = %session_id,
                        "failed to revoke auth session from security settings"
                    );
                    toast.error(error.clone());
                    pending_session.set(None);
                }
            }
        });
    });

    rsx! {
        div { class: "space-y-4",
            PasswordSettingsSection {}
            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                div { class: "flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between",
                    div {
                        h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Авторизованные устройства" }
                        p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Активные сеансы аккаунта на устройствах, где был выполнен вход." }
                    }
                    button {
                        r#type: "button",
                        disabled: is_loading || any_action_pending || loaded_session_count == 0,
                        class: all_sessions_button_class(pending_all()),
                        onclick: move |_| {
                            if is_loading || pending_all() || pending_session().is_some() {
                                return;
                            }

                            pending_all.set(true);
                            info!("revoking all auth sessions from security settings");
                            spawn(async move {
                                match sessions_api::revoke_all_sessions().await {
                                    Ok(()) => {
                                        info!("all auth sessions revoked from security settings");
                                        toast.success("Выход выполнен на всех устройствах.");
                                        let _ = navigator.replace(Route::Login {});
                                    }
                                    Err(error) => {
                                        warn!(
                                            %error,
                                            "failed to revoke all auth sessions from security settings"
                                        );
                                        toast.error(error.clone());
                                        pending_all.set(false);
                                    }
                                }
                            });
                        },
                        if pending_all() { "Выходим..." } else { "Выйти со всех устройств" }
                    }
                }

                match sessions_result {
                    None => rsx! {
                        div { class: "mt-4 space-y-2",
                            div { class: "h-[78px] animate-pulse rounded-2xl border border-zinc-800 bg-zinc-900/55" }
                            div { class: "h-[78px] animate-pulse rounded-2xl border border-zinc-800 bg-zinc-900/40" }
                        }
                    },
                    Some(Err(error)) => rsx! {
                        div { class: "mt-4 rounded-2xl border border-red-500/20 bg-red-500/10 p-4",
                            p { class: "text-[13px] font-medium text-red-100", "Не удалось загрузить активные сеансы" }
                            p { class: "mt-1 text-[12px] leading-5 text-red-200/80", "{error}" }
                            button {
                                r#type: "button",
                                class: "mt-3 flex h-9 items-center justify-center rounded-xl border border-red-400/25 bg-red-500/10 px-3 text-[12px] font-medium text-red-100 transition hover:border-red-400/40 hover:bg-red-500/15",
                                onclick: move |_| {
                                    sessions_resource.clear();
                                    sessions_resource.restart();
                                },
                                "Повторить"
                            }
                        }
                    },
                    Some(Ok(sessions)) => rsx! {
                        if sessions.is_empty() {
                            div { class: "mt-4 rounded-2xl border border-zinc-800 bg-zinc-900/45 p-5",
                                div { class: "flex max-w-lg flex-col gap-3",
                                    span { class: "flex h-10 w-10 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-zinc-500",
                                        svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 3.75 4.5 6.75v5.625c0 4.344 3.061 8.08 7.5 9.125 4.439-1.045 7.5-4.781 7.5-9.125V6.75L12 3.75Z" }
                                        }
                                    }
                                    div {
                                        p { class: "text-[13px] font-medium text-zinc-100", "Активных сеансов нет" }
                                        p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "После входа активные устройства появятся в этом списке." }
                                    }
                                }
                            }
                        } else {
                            div { class: "mt-4 space-y-2",
                                for session in sessions {
                                    {session_row(
                                        session,
                                        pending_session(),
                                        any_action_pending,
                                        on_revoke_session,
                                    )}
                                }
                            }
                        }
                    },
                }
            }
        }
    }
}

fn session_row(
    session: ActiveSession,
    pending_session_id: Option<String>,
    any_action_pending: bool,
    on_revoke: EventHandler<String>,
) -> Element {
    let session_id = session.id.clone();
    let is_pending = pending_session_id.as_deref() == Some(session.id.as_str());

    rsx! {
        div {
            key: "{session.id}",
            class: "flex flex-col gap-3 rounded-2xl border border-zinc-800 bg-zinc-900/45 p-3 sm:flex-row sm:items-center sm:justify-between",
            div { class: "min-w-0 flex items-center gap-3",
                div { class: device_icon_class(session.current),
                    svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: device_icon_path(session.client.device_kind) }
                    }
                }
                div { class: "min-w-0",
                    div { class: "flex min-w-0 items-center gap-2",
                        p { class: "truncate text-[13px] font-medium text-zinc-100", "{session_title(&session)}" }
                        if session.current {
                            span { class: "shrink-0 rounded-full border border-accent/25 bg-accent/10 px-2 py-0.5 text-[10px] font-medium text-blue-200", "текущее" }
                        }
                    }
                    p { class: "mt-0.5 truncate text-[11px] text-zinc-500",
                        "{device_kind_label(session.client.device_kind)} · активность {format_session_time(&session.last_seen_at)}"
                    }
                }
            }
            if session.current {
                span { class: "self-start rounded-xl border border-zinc-800 px-3 py-2 text-[12px] text-zinc-500 sm:self-auto", "Активно" }
            } else {
                button {
                    r#type: "button",
                    disabled: any_action_pending,
                    class: revoke_session_button_class(is_pending),
                    onclick: move |_| on_revoke.call(session_id.clone()),
                    if is_pending { "Выходим..." } else { "Выйти" }
                }
            }
        }
    }
}

fn loaded_session_count(result: &Option<Result<Vec<ActiveSession>, String>>) -> usize {
    result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .map(Vec::len)
        .unwrap_or_default()
}

fn session_title(session: &ActiveSession) -> String {
    format!(
        "{} · {}",
        session.client.os_name, session.client.browser_name
    )
}

fn device_kind_label(kind: SessionDeviceKind) -> &'static str {
    match kind {
        SessionDeviceKind::Desktop => "Компьютер",
        SessionDeviceKind::Mobile => "Телефон",
        SessionDeviceKind::Tablet => "Планшет",
        SessionDeviceKind::Bot => "Автоматический клиент",
        SessionDeviceKind::Unknown => "Устройство",
    }
}

fn format_session_time(value: &str) -> String {
    let date = value.split('T').next().unwrap_or(value);
    let time = value
        .split('T')
        .nth(1)
        .and_then(|time| time.get(..5))
        .unwrap_or_default();
    if time.is_empty() {
        return date.to_owned();
    }

    format!("{date} {time} UTC")
}

fn device_icon_class(current: bool) -> &'static str {
    if current {
        "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-accent/25 bg-accent/10 text-blue-200"
    } else {
        "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-zinc-500"
    }
}

fn device_icon_path(kind: SessionDeviceKind) -> &'static str {
    match kind {
        SessionDeviceKind::Mobile => {
            "M10.5 1.5h3A2.25 2.25 0 0 1 15.75 3.75v16.5A2.25 2.25 0 0 1 13.5 22.5h-3a2.25 2.25 0 0 1-2.25-2.25V3.75A2.25 2.25 0 0 1 10.5 1.5ZM11.25 18.75h1.5"
        }
        SessionDeviceKind::Tablet => {
            "M6.75 2.25h10.5A2.25 2.25 0 0 1 19.5 4.5v15a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5v-15A2.25 2.25 0 0 1 6.75 2.25ZM11.25 18.75h1.5"
        }
        SessionDeviceKind::Bot => {
            "M9 9h.01M15 9h.01M8.25 15h7.5M12 3v3m-5.25.75h10.5A2.25 2.25 0 0 1 19.5 9v6.75A2.25 2.25 0 0 1 17.25 18H6.75a2.25 2.25 0 0 1-2.25-2.25V9a2.25 2.25 0 0 1 2.25-2.25Z"
        }
        SessionDeviceKind::Desktop | SessionDeviceKind::Unknown => {
            "M9 17.25v1.007a3 3 0 0 1-.879 2.122L7.5 21h9l-.621-.621A3 3 0 0 1 15 18.257V17.25m-9.75 0h13.5A2.25 2.25 0 0 0 21 15V5.25A2.25 2.25 0 0 0 18.75 3H5.25A2.25 2.25 0 0 0 3 5.25V15a2.25 2.25 0 0 0 2.25 2.25Z"
        }
    }
}

fn all_sessions_button_class(is_pending: bool) -> &'static str {
    if is_pending {
        "flex h-10 w-full shrink-0 cursor-wait items-center justify-center rounded-xl border border-red-500/25 bg-red-500/15 px-3 text-[12px] font-medium text-red-100 transition disabled:opacity-60 sm:h-9 sm:w-auto"
    } else {
        "flex h-10 w-full shrink-0 items-center justify-center rounded-xl border border-red-500/25 bg-red-500/10 px-3 text-[12px] font-medium text-red-200 transition hover:border-red-500/35 hover:bg-red-500/15 disabled:cursor-not-allowed disabled:opacity-50 sm:h-9 sm:w-auto"
    }
}

fn revoke_session_button_class(is_pending: bool) -> &'static str {
    if is_pending {
        "flex h-10 w-full shrink-0 cursor-wait items-center justify-center rounded-xl border border-red-500/30 bg-red-500/10 px-3 text-[12px] font-medium text-red-100 sm:h-9 sm:w-auto"
    } else {
        "flex h-10 w-full shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-medium text-zinc-300 transition hover:border-red-500/35 hover:bg-red-500/10 hover:text-red-200 disabled:cursor-not-allowed disabled:opacity-50 sm:h-9 sm:w-auto"
    }
}

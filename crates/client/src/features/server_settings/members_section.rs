//! Server member settings section.

use dioxus::prelude::*;

use cheenhub_contracts::realtime::ServerRoleKind;

use super::kick_member_modal::KickMemberModal;
use super::member_row::MemberRow;
use super::members_data::{CustomRole, KickMemberTarget, ServerMemberRow, member_from_realtime};
use super::realtime;
use crate::features::realtime::RealtimeHandle;

/// Renders server member viewing and management controls.
#[component]
pub(crate) fn ServerMembersSettingsSection(server_id: String, server_name: String) -> Element {
    let realtime_handle = use_context::<RealtimeHandle>();
    let mut members = use_signal(|| None::<Vec<ServerMemberRow>>);
    let mut member_load_processed = use_signal(|| false);
    let mut custom_roles = use_signal(Vec::<CustomRole>::new);
    let mut roles_load_processed = use_signal(|| false);
    let mut load_error = use_signal(String::new);
    let mut refresh_requested = use_signal(|| false);
    let mut pending_kick = use_signal(|| None::<KickMemberTarget>);
    let mut kick_error = use_signal(String::new);
    let mut is_kicking = use_signal(|| false);
    // (member_id, role_id) currently being toggled
    let mut toggling_role = use_signal(|| None::<(String, String)>);
    // member_id whose role dropdown is open
    let mut open_role_menu = use_signal(|| None::<String>);

    let load_server_id = server_id.clone();
    let load_realtime_handle = realtime_handle.clone();
    let mut member_load = use_resource(move || {
        let realtime_handle = load_realtime_handle.clone();
        let request_server_id = load_server_id.clone();
        async move { realtime::list_server_members(&realtime_handle, request_server_id).await }
    });

    let roles_server_id = server_id.clone();
    let roles_realtime_handle = realtime_handle.clone();
    let mut roles_load = use_resource(move || {
        let realtime_handle = roles_realtime_handle.clone();
        let request_server_id = roles_server_id.clone();
        async move { realtime::list_server_roles(&realtime_handle, request_server_id).await }
    });

    let member_load_result = member_load.read().clone();
    use_effect(move || {
        if members().is_some() || member_load_processed() {
            return;
        }
        let Some(result) = member_load.read().clone() else {
            return;
        };
        member_load_processed.set(true);
        match result {
            Ok(response) => {
                let response_server_id = response.server_id.clone();
                members.set(Some(
                    response
                        .members
                        .into_iter()
                        .map(member_from_realtime)
                        .collect(),
                ));
                if refresh_requested() {
                    info!(server_id = %response_server_id, "refreshed server members in settings ui");
                    refresh_requested.set(false);
                }
                if !load_error.peek().is_empty() {
                    load_error.set(String::new());
                }
            }
            Err(error) => {
                warn!(%error, "failed to load server members in settings ui");
                let error_message = error.to_string();
                if load_error.peek().as_str() != error_message.as_str() {
                    load_error.set(error_message);
                }
                if refresh_requested() {
                    refresh_requested.set(false);
                }
            }
        }
    });

    use_effect(move || {
        if roles_load_processed() {
            return;
        }
        let Some(Ok(response)) = roles_load.read().clone() else {
            return;
        };
        roles_load_processed.set(true);
        let next_roles = response
            .roles
            .into_iter()
            .filter(|role| role.kind == ServerRoleKind::Custom)
            .map(|role| CustomRole {
                id: role.role_id,
                name: role.name,
                color: role.color,
            })
            .collect::<Vec<_>>();
        if custom_roles.peek().as_slice() != next_roles.as_slice() {
            custom_roles.set(next_roles);
        }
    });

    let all_members = members().unwrap_or_default();
    let is_loading = members().is_none() && member_load_result.is_none();
    let member_count = all_members.len();
    let invited_count = all_members
        .iter()
        .filter(|m| m.invite_code.is_some())
        .count();

    rsx! {
        div { class: "space-y-4",
            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                div { class: "flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between",
                    div { class: "min-w-0",
                        h3 { class: "text-[18px] font-semibold tracking-[-0.03em] text-zinc-50", "Участники" }
                        p { class: "mt-1 max-w-2xl text-[12px] leading-5 text-zinc-500",
                            "Активные участники сервера {server_name}, источник входа и действия модерации."
                        }
                    }
                    div { class: "grid grid-cols-2 gap-2 sm:flex sm:shrink-0",
                        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/70 px-3 py-2",
                            p { class: "text-[11px] text-zinc-500", "Всего" }
                            p { class: "mt-0.5 text-[16px] font-semibold text-zinc-100", "{member_count}" }
                        }
                        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/70 px-3 py-2",
                            p { class: "text-[11px] text-zinc-500", "По инвайтам" }
                            p { class: "mt-0.5 text-[16px] font-semibold text-zinc-100", "{invited_count}" }
                        }
                    }
                }
            }

            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                div { class: "flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between",
                    div {
                        h4 { class: "text-[15px] font-semibold text-zinc-50", "Список участников" }
                        p { class: "mt-1 text-[12px] leading-5 text-zinc-500",
                            "Инвайт показывает ссылку, через которую пользователь присоединился в последний раз."
                        }
                    }
                    button {
                        r#type: "button",
                        disabled: is_loading || is_kicking(),
                        class: refresh_button_class(is_loading || is_kicking()),
                        onclick: move |_| {
                            if is_loading || is_kicking() {
                                return;
                            }
                            load_error.set(String::new());
                            refresh_requested.set(true);
                            member_load_processed.set(false);
                            roles_load_processed.set(false);
                            members.set(None);
                            custom_roles.set(Vec::new());
                            member_load.clear();
                            member_load.restart();
                            roles_load.clear();
                            roles_load.restart();
                            info!("refreshing server members in settings ui");
                        },
                        svg { class: if is_loading { "h-4 w-4 animate-spin" } else { "h-4 w-4" }, fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99" }
                        }
                        "Обновить"
                    }
                }

                if is_loading {
                    div { class: "mt-4 space-y-2",
                        div { class: "h-[76px] animate-pulse rounded-2xl border border-zinc-800 bg-zinc-900/55" }
                        div { class: "h-[76px] animate-pulse rounded-2xl border border-zinc-800 bg-zinc-900/40" }
                        div { class: "h-[76px] animate-pulse rounded-2xl border border-zinc-800 bg-zinc-900/30" }
                    }
                } else if !load_error().is_empty() && members().is_none() {
                    div { class: "mt-4 rounded-2xl border border-red-500/20 bg-red-500/10 p-4",
                        p { class: "text-[13px] font-medium text-red-100", "Не удалось загрузить участников" }
                        p { class: "mt-1 text-[12px] leading-5 text-red-200", "{load_error()}" }
                    }
                } else if all_members.is_empty() {
                    div { class: "mt-4 rounded-2xl border border-zinc-800 bg-zinc-900/45 p-5",
                        p { class: "text-[13px] font-medium text-zinc-100", "На сервере пока нет участников" }
                        p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Создай инвайт, чтобы пригласить первых людей." }
                    }
                } else {
                    div { class: "mt-4 rounded-2xl border border-zinc-800",
                        div { class: "grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(120px,.6fr)_160px] gap-3 rounded-t-2xl border-b border-zinc-800 bg-zinc-900/70 px-4 py-2 text-[10px] font-medium uppercase tracking-[0.14em] text-zinc-600",
                            span { "Участник" }
                            span { "Инвайт" }
                            span { "Вступил" }
                            span { class: "text-right", "Действия" }
                        }
                        div { class: "divide-y divide-zinc-800 rounded-b-2xl bg-zinc-950/45",
                            for member in all_members {
                                MemberRow {
                                    key: "{member.id}",
                                    member: member.clone(),
                                    custom_roles: custom_roles(),
                                    is_kicking: is_kicking(),
                                    toggling_role: toggling_role(),
                                    role_menu_open: open_role_menu().as_deref() == Some(&member.id),
                                    on_open_role_menu: {
                                        let member_id = member.id.clone();
                                        move |_| {
                                            if open_role_menu().as_deref() == Some(&member_id) {
                                                open_role_menu.set(None);
                                            } else {
                                                open_role_menu.set(Some(member_id.clone()));
                                            }
                                        }
                                    },
                                    on_close_role_menu: move |_| open_role_menu.set(None),
                                    on_toggle_role: {
                                        let action_realtime = realtime_handle.clone();
                                        let action_server_id = server_id.clone();
                                        let member_id = member.id.clone();
                                        move |(role_id, currently_has): (String, bool)| {
                                            if toggling_role().is_some() {
                                                return;
                                            }
                                            let key = (member_id.clone(), role_id.clone());
                                            toggling_role.set(Some(key.clone()));
                                            let realtime = action_realtime.clone();
                                            let sid = action_server_id.clone();
                                            let uid = member_id.clone();
                                            let rid = role_id.clone();
                                            spawn(async move {
                                                if currently_has {
                                                    match realtime::revoke_server_member_role(
                                                        &realtime, sid, uid.clone(), rid.clone(),
                                                    )
                                                    .await
                                                    {
                                                        Ok(_) => {
                                                            members.set(Some(
                                                                members()
                                                                    .unwrap_or_default()
                                                                    .into_iter()
                                                                    .map(|mut m| {
                                                                        if m.id == uid {
                                                                            m.role_ids.retain(|r| r != &rid);
                                                                        }
                                                                        m
                                                                    })
                                                                    .collect(),
                                                            ));
                                                            info!(user_id = %uid, role_id = %rid, "revoked role");
                                                        }
                                                        Err(e) => warn!(%e, "failed to revoke role"),
                                                    }
                                                } else {
                                                    match realtime::assign_server_member_role(
                                                        &realtime, sid, uid.clone(), rid.clone(),
                                                    )
                                                    .await
                                                    {
                                                        Ok(_) => {
                                                            members.set(Some(
                                                                members()
                                                                    .unwrap_or_default()
                                                                    .into_iter()
                                                                    .map(|mut m| {
                                                                        if m.id == uid && !m.role_ids.contains(&rid) {
                                                                            m.role_ids.push(rid.clone());
                                                                        }
                                                                        m
                                                                    })
                                                                    .collect(),
                                                            ));
                                                            info!(user_id = %uid, role_id = %rid, "assigned role");
                                                        }
                                                        Err(e) => warn!(%e, "failed to assign role"),
                                                    }
                                                }
                                                toggling_role.set(None);
                                            });
                                        }
                                    },
                                    on_kick: {
                                        let member_id = member.id.clone();
                                        let member_name = member.name.clone();
                                        let member_is_owner = member.is_owner;
                                        move |_| {
                                            if member_is_owner || is_kicking() {
                                                return;
                                            }
                                            kick_error.set(String::new());
                                            open_role_menu.set(None);
                                            pending_kick.set(Some(KickMemberTarget {
                                                id: member_id.clone(),
                                                name: member_name.clone(),
                                            }));
                                            info!(member_id = %member_id, "opened server member kick modal in settings ui");
                                        }
                                    },
                                }
                            }
                        }
                    }
                }
            }

            if let Some(member) = pending_kick() {
                KickMemberModal {
                    member: member.clone(),
                    is_busy: is_kicking(),
                    error: kick_error(),
                    on_cancel: move |_| {
                        if !is_kicking() {
                            pending_kick.set(None);
                            kick_error.set(String::new());
                        }
                    },
                    on_confirm: {
                        let action_realtime_handle = realtime_handle.clone();
                        let action_server_id = server_id.clone();
                        move |duration| {
                            if is_kicking() {
                                return;
                            }
                            let Some(target) = pending_kick() else {
                                return;
                            };
                            is_kicking.set(true);
                            kick_error.set(String::new());
                            let action_realtime = action_realtime_handle.clone();
                            let action_server_id = action_server_id.clone();
                            let target_id = target.id.clone();
                            let target_name = target.name.clone();
                            spawn(async move {
                                match realtime::kick_server_member(
                                    &action_realtime,
                                    action_server_id,
                                    target_id.clone(),
                                    duration,
                                )
                                .await
                                {
                                    Ok(response) => {
                                        members.set(Some(
                                            members()
                                                .unwrap_or_default()
                                                .into_iter()
                                                .filter(|member| member.id != response.user_id)
                                                .collect(),
                                        ));
                                        pending_kick.set(None);
                                        is_kicking.set(false);
                                        info!(
                                            member_id = %response.user_id,
                                            excluded_until = response.excluded_until.as_deref().unwrap_or(""),
                                            "kicked server member in settings ui"
                                        );
                                    }
                                    Err(error) => {
                                        is_kicking.set(false);
                                        kick_error.set(error.to_string());
                                        warn!(
                                            %error,
                                            member_id = %target_id,
                                            member_name = %target_name,
                                            "failed to kick server member in settings ui"
                                        );
                                    }
                                }
                            });
                        }
                    },
                }
            }
        }
    }
}

fn refresh_button_class(disabled: bool) -> &'static str {
    if disabled {
        "flex min-h-9 cursor-wait items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900/50 px-3 text-[12px] font-medium text-zinc-500"
    } else {
        "flex min-h-9 items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900/70 px-3 text-[12px] font-medium text-zinc-300 transition hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100"
    }
}

//! Server member settings section.

use dioxus::prelude::*;

use cheenhub_contracts::realtime::ServerRoleKind;

use super::kick_member_modal::KickMemberModal;
use super::members_data::{KickMemberTarget, ServerMemberRow, member_from_realtime};
use super::realtime;
use crate::features::realtime::RealtimeHandle;

/// Compact custom role summary used in the members section.
#[derive(Clone, PartialEq)]
struct CustomRole {
    id: String,
    name: String,
    color: String,
}

/// Renders server member viewing and management controls.
#[component]
pub(crate) fn ServerMembersSettingsSection(server_id: String, server_name: String) -> Element {
    let realtime_handle = use_context::<RealtimeHandle>();
    let mut members = use_signal(|| None::<Vec<ServerMemberRow>>);
    let mut custom_roles = use_signal(|| Vec::<CustomRole>::new());
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
        if members().is_some() {
            return;
        }
        let Some(result) = member_load.read().clone() else {
            return;
        };
        match result {
            Ok(response) => {
                members.set(Some(
                    response.members.into_iter().map(member_from_realtime).collect(),
                ));
                if refresh_requested() {
                    info!(server_id = %response.server_id, "refreshed server members in settings ui");
                }
                refresh_requested.set(false);
                load_error.set(String::new());
            }
            Err(error) => {
                warn!(%error, "failed to load server members in settings ui");
                load_error.set(error.to_string());
                refresh_requested.set(false);
            }
        }
    });

    use_effect(move || {
        if !custom_roles().is_empty() {
            return;
        }
        let Some(Ok(response)) = roles_load.read().clone() else {
            return;
        };
        custom_roles.set(
            response
                .roles
                .into_iter()
                .filter(|role| role.kind == ServerRoleKind::Custom)
                .map(|role| CustomRole {
                    id: role.role_id,
                    name: role.name,
                    color: role.color,
                })
                .collect(),
        );
    });

    let all_members = members().unwrap_or_default();
    let is_loading = members().is_none() && member_load_result.is_none();
    let member_count = all_members.len();
    let invited_count = all_members.iter().filter(|m| m.invite_code.is_some()).count();

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

#[component]
fn MemberRow(
    member: ServerMemberRow,
    custom_roles: Vec<CustomRole>,
    is_kicking: bool,
    toggling_role: Option<(String, String)>,
    role_menu_open: bool,
    on_open_role_menu: EventHandler<()>,
    on_close_role_menu: EventHandler<()>,
    on_toggle_role: EventHandler<(String, bool)>,
    on_kick: EventHandler<()>,
) -> Element {
    let has_custom_roles = !custom_roles.is_empty();

    rsx! {
        div { class: "relative grid grid-cols-1 gap-3 px-4 py-3 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(120px,.6fr)_160px] sm:items-center",

            // Participant column
            div { class: "min-w-0",
                div { class: "flex min-w-0 flex-wrap items-center gap-2",
                    p { class: "truncate text-[13px] font-medium text-zinc-100", "{member.name}" }
                    if member.is_owner {
                        span { class: "rounded-full border border-accent/25 bg-accent/10 px-2 py-0.5 text-[10px] font-medium text-blue-200",
                            "Владелец"
                        }
                    }
                    // Role color dots
                    if !member.role_ids.is_empty() {
                        div { class: "flex items-center gap-1",
                            for role_id in member.role_ids.iter() {
                                if let Some(role) = custom_roles.iter().find(|r| &r.id == role_id) {
                                    span {
                                        key: "{role_id}",
                                        title: "{role.name}",
                                        class: "inline-flex items-center gap-1 rounded-full border border-zinc-700/50 bg-zinc-800/70 px-1.5 py-0.5 text-[10px] font-medium",
                                        span {
                                            class: "h-1.5 w-1.5 rounded-full flex-shrink-0",
                                            style: "background-color: {role.color}",
                                        }
                                        span { class: "text-zinc-300", "{role.name}" }
                                    }
                                }
                            }
                        }
                    }
                }
                p { class: "mt-0.5 truncate font-mono text-[10px] text-zinc-600", "{member.id}" }
            }

            // Invite column
            div { class: "min-w-0",
                if let Some(invite_code) = member.invite_code.clone() {
                    p { class: "truncate font-mono text-[12px] text-zinc-300", "{invite_code}" }
                    if let Some(used_at) = member.invite_used_at.clone() {
                        p { class: "mt-0.5 truncate text-[10px] text-zinc-600", "{used_at}" }
                    }
                } else {
                    p { class: "text-[12px] text-zinc-500", "Без инвайта" }
                }
            }

            // Join date column
            p { class: "text-[12px] text-zinc-500", "{member.joined_at}" }

            // Actions column
            div { class: "flex items-center justify-start gap-2 sm:justify-end",
                if has_custom_roles && !member.is_owner {
                    div { class: "relative",
                        button {
                            r#type: "button",
                            disabled: is_kicking || toggling_role.is_some(),
                            class: role_button_class(is_kicking || toggling_role.is_some()),
                            onclick: move |_| on_open_role_menu(()),
                            svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.568 3H5.25A2.25 2.25 0 0 0 3 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 0 0 5.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 0 0 9.568 3Z" }
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 6h.008v.008H6V6Z" }
                            }
                            "Роли"
                        }

                        if role_menu_open {
                            div {
                                class: "fixed inset-0 z-40",
                                onclick: move |_| on_close_role_menu(()),
                            }
                            div {
                                class: "absolute right-0 top-full z-50 mt-1 w-52 rounded-xl border border-zinc-700 bg-zinc-900 py-1 shadow-xl",
                                onclick: move |e| e.stop_propagation(),

                                div { class: "border-b border-zinc-800 px-3 py-2",
                                    p { class: "text-[11px] font-medium text-zinc-400", "Роли участника" }
                                }

                                for role in custom_roles.iter() {
                                    {
                                        let has_role = member.role_ids.contains(&role.id);
                                        let is_toggling = toggling_role.as_ref()
                                            .is_some_and(|(uid, rid)| uid == &member.id && rid == &role.id);
                                        let role_id = role.id.clone();
                                        let role_color = role.color.clone();
                                        let role_name = role.name.clone();
                                        rsx! {
                                            button {
                                                key: "{role_id}",
                                                r#type: "button",
                                                disabled: is_toggling,
                                                class: role_item_class(is_toggling),
                                                onclick: move |_| on_toggle_role((role_id.clone(), has_role)),
                                                div { class: "flex items-center gap-2 flex-1 min-w-0",
                                                    span {
                                                        class: "h-2.5 w-2.5 rounded-full flex-shrink-0",
                                                        style: "background-color: {role_color}",
                                                    }
                                                    span { class: "truncate text-[12px] text-zinc-200", "{role_name}" }
                                                }
                                                if is_toggling {
                                                    svg { class: "h-3.5 w-3.5 animate-spin text-zinc-500 flex-shrink-0", fill: "none", view_box: "0 0 24 24",
                                                        circle { class: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", stroke_width: "4" }
                                                        path { class: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4Z" }
                                                    }
                                                } else if has_role {
                                                    svg { class: "h-3.5 w-3.5 text-blue-400 flex-shrink-0", fill: "none", stroke: "currentColor", stroke_width: "2.5", view_box: "0 0 24 24",
                                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "m4.5 12.75 6 6 9-13.5" }
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

                button {
                    r#type: "button",
                    disabled: member.is_owner || is_kicking,
                    class: kick_button_class(member.is_owner || is_kicking),
                    onclick: move |_| on_kick(()),
                    "Исключить"
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

fn role_button_class(disabled: bool) -> &'static str {
    if disabled {
        "flex h-9 cursor-not-allowed items-center gap-1.5 rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-medium text-zinc-500"
    } else {
        "flex h-9 items-center gap-1.5 rounded-xl border border-zinc-700/60 bg-zinc-800/60 px-3 text-[12px] font-medium text-zinc-300 transition hover:border-zinc-600 hover:bg-zinc-800 hover:text-zinc-100"
    }
}

fn role_item_class(disabled: bool) -> &'static str {
    if disabled {
        "flex w-full cursor-wait items-center gap-2 px-3 py-2 text-left opacity-60"
    } else {
        "flex w-full items-center gap-2 px-3 py-2 text-left transition hover:bg-zinc-800"
    }
}

fn kick_button_class(disabled: bool) -> &'static str {
    if disabled {
        "flex h-9 cursor-not-allowed items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-medium text-zinc-500"
    } else {
        "flex h-9 items-center justify-center rounded-xl border border-red-500/25 bg-red-500/10 px-3 text-[12px] font-medium text-red-200 transition hover:border-red-500/35 hover:bg-red-500/15"
    }
}

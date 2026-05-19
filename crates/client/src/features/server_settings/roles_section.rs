//! Server role settings section.
use super::realtime;
use super::role_permissions_card::RolePermissionsCard;
use super::role_save_bar::RoleSaveBar;
use super::roles_data::{
    OWNER_ROLE_ID, ROLE_COLORS, RoleDraft, RolePermission, color_button_class, delete_button_class,
    hex_input_class, hex_to_rgb, initial_roles, is_hex_color, non_empty_role_name,
    normalize_hex_input, preview_color, role_card_class, role_initial, roles_from_realtime,
    roles_to_realtime, selected_role, update_selected_role,
};
use crate::features::realtime::RealtimeHandle;
use dioxus::prelude::*;
/// Renders a role management UI.
#[component]
pub(crate) fn ServerRolesSettingsSection(
    server_id: String,
    server_name: String,
    is_owner: bool,
) -> Element {
    let realtime_handle = use_context::<RealtimeHandle>();
    let mut roles = use_signal(|| None::<Vec<RoleDraft>>);
    let mut selected_role_id = use_signal(|| OWNER_ROLE_ID.to_owned());
    let mut role_search = use_signal(String::new);
    let mut dirty = use_signal(|| false);
    let mut load_error = use_signal(String::new);
    let mut save_error = use_signal(String::new);
    let mut is_saving = use_signal(|| false);
    let load_server_id = server_id.clone();
    let load_realtime_handle = realtime_handle.clone();
    let mut role_load = use_resource(move || {
        let realtime_handle = load_realtime_handle.clone();
        let request_server_id = load_server_id.clone();
        async move { realtime::list_server_roles(&realtime_handle, request_server_id).await }
    });
    let role_load_result = role_load.read().clone();
    use_effect(move || {
        if roles().is_some() {
            return;
        }
        let Some(result) = role_load.read().clone() else {
            return;
        };

        match result {
            Ok(response) => {
                let loaded_roles = roles_from_realtime(response.roles);
                let selected = loaded_roles
                    .iter()
                    .find(|role| role.is_owner())
                    .or_else(|| loaded_roles.first())
                    .map(|role| role.id.clone())
                    .unwrap_or_else(|| OWNER_ROLE_ID.to_owned());
                selected_role_id.set(selected);
                roles.set(Some(loaded_roles));
                load_error.set(String::new());
                info!(
                    server_id = %response.server_id,
                    "loaded server roles in settings ui"
                );
            }
            Err(error) => {
                warn!(%error, "failed to load server roles in settings ui");
                load_error.set(error.to_string());
            }
        }
    });
    let all_roles = roles().unwrap_or_default();
    let is_loading = roles().is_none() && role_load_result.is_none();
    let role = selected_role(&all_roles, &selected_role_id())
        .cloned()
        .unwrap_or_else(|| initial_roles().remove(0));
    let visible_roles = all_roles
        .iter()
        .filter(|role| {
            let query = role_search().trim().to_lowercase();
            query.is_empty() || role.name.to_lowercase().contains(&query)
        })
        .cloned()
        .collect::<Vec<_>>();
    let custom_hex = role.color.clone();
    let custom_hex_valid = is_hex_color(&custom_hex);
    let selected_rgb = hex_to_rgb(&role.color);
    let role_initial_text = role_initial(&role.name);
    let selected_role_name = role.name.clone();
    let selected_role_color = role.color.clone();
    let selected_role_locked = role.is_required;
    let selected_role_owner = role.is_owner();
    let perm_locked = selected_role_owner || !is_owner;
    let edit_locked = !is_owner;
    rsx! {
        div { class: "role-settings-panel space-y-4 pb-24 xl:pb-0",
            div { class: "role-editor-surface rounded-2xl border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)] transition-[border-color,background,box-shadow] duration-200 hover:border-zinc-700/80 hover:bg-zinc-950/80",
                div { class: "flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between",
                    div { class: "min-w-0",
                        h3 { class: "text-[20px] font-semibold tracking-[-0.04em] text-zinc-50", "Редактирование ролей" }
                        p { class: "mt-1 max-w-2xl text-[13px] leading-5 text-zinc-500",
                            "Настраивай права доступа, цвет и отображение ролей на сервере {server_name}."
                        }
                    }
                }
            }

            div { class: "grid gap-4 xl:grid-cols-[320px_minmax(0,1fr)]",
                aside { class: "space-y-4",
                        div { class: "role-editor-surface rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4 shadow-[0_18px_60px_rgba(0,0,0,.16)]",
                        div { class: "mb-4 flex items-start justify-between gap-3",
                            div {
                                h4 { class: "text-[15px] font-semibold text-zinc-50", "Роли сервера" }
                                p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Выбери роль, чтобы изменить название, цвет и права." }
                            }
                            button {
                                r#type: "button",
                                disabled: edit_locked,
                                class: if edit_locked { "grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-accent/40 text-white/40 cursor-not-allowed" } else { "grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-accent text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_4px_18px_rgba(59,130,246,0.16)] transition-[background,box-shadow,transform] duration-150 hover:-translate-y-px hover:bg-blue-400 hover:shadow-[0_0_0_4px_rgba(59,130,246,0.14),0_10px_28px_rgba(59,130,246,0.18)]" },
                                "aria-label": "Создать роль",
                                onclick: move |_| {
                                    if edit_locked {
                                        return;
                                    }
                                    let Some(mut next_roles) = roles() else {
                                        return;
                                    };
                                    let next_number = next_roles.len() + 1;
                                    let new_role = RoleDraft {
                                        id: format!("8fb4df0c-9da5-4a8a-b49f-{next_number:012}"),
                                        name: format!("Новая роль {next_number}"),
                                        color: "#a855f7".to_owned(),
                                        members: 0,
                                        is_required: false,
                                        kind: cheenhub_contracts::realtime::ServerRoleKind::Custom,
                                        permissions: vec![RolePermission::CreateInviteLinks],
                                    };
                                    let new_role_id = new_role.id.clone();
                                    let member_index = next_roles
                                        .iter()
                                        .position(|role| role.kind == cheenhub_contracts::realtime::ServerRoleKind::Member)
                                        .unwrap_or(next_roles.len());
                                    next_roles.insert(member_index, new_role);
                                    roles.set(Some(next_roles));
                                    selected_role_id.set(new_role_id.clone());
                                    dirty.set(true);
                                    info!(role_id = %new_role_id, "created server role in settings ui");
                                },
                                svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                                    path { stroke_linecap: "round", d: "M12 5v14M5 12h14" }
                                }
                            }
                        }
                        label { class: "mb-3 flex items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900/80 px-3 py-2.5 text-[13px] text-zinc-500 transition-[background,border-color,box-shadow] duration-150 focus-within:border-zinc-700 focus-within:bg-zinc-900 focus-within:shadow-[0_0_0_4px_rgba(255,255,255,0.03)]",
                            svg { class: "h-4 w-4 shrink-0", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", d: "m21 21-4.3-4.3M11 19a8 8 0 1 1 0-16 8 8 0 0 1 0 16Z" }
                            }
                            input {
                                r#type: "search",
                                value: role_search(),
                                placeholder: "Найти роль",
                                oninput: move |event| role_search.set(event.value()),
                                class: "w-full bg-transparent text-zinc-200 outline-none placeholder:text-zinc-600",
                            }
                        }
                        div { class: "space-y-2",
                            if is_loading {
                                div { class: "h-[72px] animate-pulse rounded-2xl border border-zinc-800 bg-zinc-900/55" }
                                div { class: "h-[72px] animate-pulse rounded-2xl border border-zinc-800 bg-zinc-900/40" }
                                div { class: "h-[72px] animate-pulse rounded-2xl border border-zinc-800 bg-zinc-900/30" }
                            } else if !load_error().is_empty() && roles().is_none() {
                                div { class: "rounded-2xl border border-red-500/20 bg-red-500/10 p-4",
                                    p { class: "text-[13px] font-medium text-red-100", "Не удалось загрузить роли" }
                                    p { class: "mt-1 text-[12px] leading-5 text-red-200", "{load_error()}" }
                                }
                            } else if visible_roles.is_empty() {
                                div { class: "rounded-2xl border border-zinc-800 bg-zinc-900/45 p-4 text-[12px] text-zinc-500",
                                    "Роли с таким названием не найдены."
                                }
                            } else {
                                for list_role in visible_roles {
                                    button {
                                        key: "{list_role.id}",
                                        r#type: "button",
                                        class: role_card_class(selected_role_id().as_str() == list_role.id.as_str()),
                                        "aria-selected": selected_role_id().as_str() == list_role.id.as_str(),
                                        onclick: {
                                            let role_id = list_role.id.clone();
                                            move |_| {
                                                selected_role_id.set(role_id.clone());
                                                info!(role_id = %role_id, "selected server role in settings ui");
                                            }
                                        },
                                        div { class: "flex items-center gap-3",
                                            span {
                                                class: "grid h-10 w-10 shrink-0 place-items-center rounded-xl text-sm font-semibold",
                                                style: "background: rgba({hex_to_rgb(&list_role.color)}, .10); color: {list_role.color}; border: 1px solid rgba({hex_to_rgb(&list_role.color)}, .24);",
                                                "{role_initial(&list_role.name)}"
                                            }
                                            span { class: "min-w-0 flex-1",
                                                span { class: "flex items-center gap-2",
                                                    span { class: "truncate text-[13px] font-semibold text-zinc-100", "{list_role.name}" }
                                                    if list_role.is_required {
                                                        span { class: "rounded-full border border-zinc-800 bg-zinc-950 px-1.5 py-0.5 text-[10px] text-zinc-600", "sys" }
                                                    }
                                                }
                                            }
                                        }
                                        if selected_role_id().as_str() == list_role.id.as_str() {
                                            div { class: "role-details mt-3 grid grid-cols-2 gap-2 border-t border-zinc-800/60 pt-3 text-[12px]",
                                                div { class: "role-stat-tile rounded-xl border border-zinc-800 bg-zinc-950/60 px-3 py-2",
                                                    span { class: "block text-[10px] uppercase tracking-[0.16em] text-zinc-600", "Участников" }
                                                    span { class: "mt-1 block font-medium text-zinc-200", "{list_role.members}" }
                                                }
                                                div { class: "role-stat-tile rounded-xl border border-zinc-800 bg-zinc-950/60 px-3 py-2",
                                                    span { class: "block text-[10px] uppercase tracking-[0.16em] text-zinc-600", "Прав" }
                                                    span { class: "mt-1 block font-medium text-zinc-200", "{list_role.effective_permissions().len()}" }
                                                }
                                                div { class: "role-stat-tile rounded-xl border border-zinc-800 bg-zinc-950/60 px-3 py-2",
                                                    span { class: "block text-[10px] uppercase tracking-[0.16em] text-zinc-600", "Цвет" }
                                                    span { class: "mt-1 block font-mono text-[11px] font-medium text-zinc-200", "{list_role.color}" }
                                                }
                                                div { class: "role-stat-tile rounded-xl border border-zinc-800 bg-zinc-950/60 px-3 py-2",
                                                    span { class: "block text-[10px] uppercase tracking-[0.16em] text-zinc-600", "Удаление" }
                                                    span { class: "mt-1 block font-medium text-zinc-200",
                                                        if list_role.is_required { "Запрещено" } else { "Доступно" }
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
                section { class: "min-w-0 space-y-4",
                    div { key: "{role.id}:hero", class: "role-editor-surface rounded-2xl border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)] transition-[border-color,background,box-shadow] duration-200 hover:border-zinc-700/80 hover:bg-zinc-950/80",
                        div { class: "flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between",
                            div { class: "flex min-w-0 items-center gap-4",
                                div {
                                    class: "grid h-14 w-14 shrink-0 place-items-center rounded-2xl text-xl font-semibold",
                                    style: "background: rgba({selected_rgb}, .10); color: {selected_role_color}; border: 1px solid rgba({selected_rgb}, .24);",
                                    "{role_initial_text}"
                                }
                                div { class: "min-w-0",
                                    div { class: "mb-1 flex flex-wrap items-center gap-2",
                                        h4 { class: "truncate text-xl font-semibold tracking-[-0.04em] text-zinc-50", "{selected_role_name}" }
                                        if selected_role_locked {
                                            span { class: "rounded-full border border-zinc-700 bg-zinc-950 px-2 py-0.5 text-[10px] uppercase tracking-[0.2em] text-zinc-500", "системная" }
                                        }
                                    }
                                    p { class: "text-[13px] leading-5 text-zinc-500", "{role.description()}" }
                                }
                            }
                            button {
                                r#type: "button",
                                disabled: selected_role_locked || edit_locked,
                                class: delete_button_class(selected_role_locked || edit_locked),
                                onclick: move |_| {
                                    if selected_role_locked || edit_locked {
                                        return;
                                    }
                                    let current_id = selected_role_id();
                                    let Some(mut next_roles) = roles() else {
                                        return;
                                    };
                                    next_roles.retain(|role| role.id != current_id);
                                    let next_selected = next_roles
                                        .iter()
                                        .find(|role| role.kind != cheenhub_contracts::realtime::ServerRoleKind::Member)
                                        .or_else(|| next_roles.first())
                                        .map(|role| role.id.clone())
                                        .unwrap_or_else(|| OWNER_ROLE_ID.to_owned());
                                    roles.set(Some(next_roles));
                                    selected_role_id.set(next_selected);
                                    dirty.set(true);
                                    info!(role_id = %current_id, "deleted server role in settings ui");
                                },
                                "Удалить"
                            }
                        }
                    }
                    div { class: "space-y-4",
                            div { key: "{role.id}:base", class: "role-editor-surface rounded-2xl border border-zinc-800 bg-zinc-950/70 p-5 transition-[border-color,background,box-shadow] duration-200 hover:border-zinc-700/80 hover:bg-zinc-950/80",
                                div { class: "mb-5",
                                    div {
                                        h5 { class: "text-sm font-semibold text-zinc-50", "Основные настройки" }
                                        p { class: "mt-1 text-[13px] leading-5 text-zinc-500", "Название и цвет роли в списках участников." }
                                    }
                                }
                                div { class: "grid gap-4",
                                    label { class: "block",
                                        span { class: "mb-2 block text-[12px] font-medium text-zinc-500", "Название роли" }
                                        input {
                                            r#type: "text",
                                            value: "{role.name}",
                                            maxlength: "32",
                                            disabled: edit_locked,
                                            oninput: move |event| {
                                                if edit_locked { return; }
                                                let value = non_empty_role_name(event.value());
                                                update_selected_role(&mut roles, &selected_role_id(), |role| {
                                                    role.name = value.clone();
                                                });
                                                dirty.set(true);
                                            },
                                            class: if edit_locked { "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-900/40 px-3 text-[13px] text-zinc-500 outline-none cursor-not-allowed" } else { "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-900 px-3 text-[13px] text-zinc-200 outline-none transition-[background,border-color,box-shadow] duration-150 placeholder:text-zinc-600 focus:border-zinc-700 focus:shadow-[0_0_0_4px_rgba(59,130,246,0.08)]" },
                                        }
                                    }
                                }

                                div { class: "mt-5",
                                    div { class: "mb-2 flex items-center justify-between gap-3",
                                        span { class: "text-[12px] font-medium text-zinc-500", "Цвет роли" }
                                    }
                                    div { class: "flex flex-wrap gap-2",
                                        for color in ROLE_COLORS {
                                            button {
                                                key: "{color}",
                                                r#type: "button",
                                                disabled: edit_locked,
                                                class: if edit_locked { "grid h-9 w-9 place-items-center rounded-xl border border-zinc-800 bg-zinc-900 opacity-40 cursor-not-allowed" } else { color_button_class(role.color.eq_ignore_ascii_case(color)) },
                                                "aria-label": "Выбрать цвет {color}",
                                                onclick: {
                                                    let next_color = (*color).to_owned();
                                                    move |_| {
                                                        if edit_locked { return; }
                                                        update_selected_role(&mut roles, &selected_role_id(), |role| {
                                                            role.color = next_color.clone();
                                                        });
                                                        dirty.set(true);
                                                        info!(color = %next_color, "changed server role color from palette");
                                                    }
                                                },
                                                span { class: "h-5 w-5 rounded-lg", style: "background: {color};" }
                                            }
                                        }
                                    }
                                    label { class: "mt-4 block",
                                        span { class: "mb-2 block text-[12px] font-medium text-zinc-500", "Hex значение" }
                                        div { class: "flex items-center gap-3",
                                            input {
                                                r#type: "text",
                                                value: "{custom_hex}",
                                                maxlength: "7",
                                                placeholder: "#3b82f6",
                                                disabled: edit_locked,
                                                oninput: move |event| {
                                                    if edit_locked { return; }
                                                    let value = normalize_hex_input(event.value());
                                                    update_selected_role(&mut roles, &selected_role_id(), |role| {
                                                        role.color = value.clone();
                                                    });
                                                    dirty.set(true);
                                                },
                                                class: if edit_locked { "h-11 min-w-0 flex-1 rounded-xl border border-zinc-800 bg-zinc-900/40 px-3 font-mono text-[13px] text-zinc-500 outline-none cursor-not-allowed" } else { hex_input_class(custom_hex_valid) },
                                            }
                                            span {
                                                class: "role-color-preview h-11 w-11 shrink-0 rounded-xl border border-zinc-800",
                                                style: "background: {preview_color(&custom_hex)};",
                                                "aria-label": "Превью цвета роли",
                                            }
                                        }
                                        if !custom_hex_valid {
                                            p { class: "mt-2 text-[11px] leading-4 text-red-200", "Используй формат #RRGGBB." }
                                        }
                                    }
                                }
                            }
                            RolePermissionsCard {
                                key: "{role.id}:permissions",
                                role: role.clone(),
                                perm_locked,
                                selected_role_owner,
                                is_owner,
                                on_permission_change: move |(permission, checked): (RolePermission, bool)| {
                                    update_selected_role(&mut roles, &selected_role_id(), |role| {
                                        role.set_permission(permission, checked);
                                    });
                                    dirty.set(true);
                                    info!(
                                        permission = permission.key(),
                                        enabled = checked,
                                        "changed server role permission in settings ui"
                                    );
                                },
                            }
                    }
                }
            }
            RoleSaveBar {
                dirty: dirty(),
                save_error: save_error(),
                is_saving: is_saving(),
                can_save: custom_hex_valid,
                on_reset: move |_| {
                    roles.set(None);
                    dirty.set(false);
                    save_error.set(String::new());
                    role_load.clear();
                    role_load.restart();
                    // TODO: integrate the shared toast system for reset feedback.
                    info!("reset server role changes in settings ui");
                },
                on_save: move |_| {
                    let Some(current_roles) = roles() else {
                        return;
                    };
                    is_saving.set(true);
                    save_error.set(String::new());
                    let save_realtime = realtime_handle.clone();
                    let save_server_id = server_id.clone();
                    spawn(async move {
                        match realtime::save_server_roles(
                            &save_realtime,
                            save_server_id,
                            roles_to_realtime(current_roles),
                        )
                        .await
                        {
                            Ok(response) => {
                                let saved_roles = roles_from_realtime(response.roles);
                                let selected = selected_role_id();
                                let next_selected = saved_roles
                                    .iter()
                                    .find(|role| role.id == selected)
                                    .or_else(|| saved_roles.iter().find(|role| role.is_owner()))
                                    .or_else(|| saved_roles.first())
                                    .map(|role| role.id.clone())
                                    .unwrap_or_else(|| OWNER_ROLE_ID.to_owned());
                                roles.set(Some(saved_roles));
                                selected_role_id.set(next_selected);
                                is_saving.set(false);
                                dirty.set(false);
                                // TODO: integrate the shared toast system for save feedback.
                                info!(
                                    server_id = %response.server_id,
                                    "saved server role changes in settings ui"
                                );
                            }
                            Err(error) => {
                                warn!(%error, "failed to save server role changes in settings ui");
                                is_saving.set(false);
                                save_error.set(error.to_string());
                            }
                        }
                    });
                },
            }
        }
    }
}

//! Поле выбора ролей, которым разрешено писать в комнату.

use cheenhub_contracts::realtime::ServerRoleSummary;
use cheenhub_contracts::rest::ServerRoomWriteAccessMode;
use dioxus::prelude::*;

/// Обработчик смены режима доступа к записи.
pub(super) type WriteAccessModeHandler = Callback<ServerRoomWriteAccessMode>;
/// Обработчик переключения роли в списке доступа.
pub(super) type WriteRoleToggleHandler = Callback<String>;

/// Рендерит выбор режима доступа к записи и мультивыбор ролей.
pub(super) fn write_access_field(
    mode: ServerRoomWriteAccessMode,
    selected_role_ids: &[String],
    roles: &[ServerRoleSummary],
    on_mode_change: WriteAccessModeHandler,
    on_toggle_role: WriteRoleToggleHandler,
) -> Element {
    let roles_selected_mode = mode == ServerRoomWriteAccessMode::SelectedRoles;
    let warning_visible = roles_selected_mode && selected_role_ids.is_empty();
    let warning_class = if warning_visible {
        "grid-rows-[1fr] opacity-100"
    } else {
        "grid-rows-[0fr] opacity-0"
    };
    let roles_class = if roles_selected_mode {
        "grid-rows-[1fr] opacity-100"
    } else {
        "grid-rows-[0fr] opacity-0"
    };

    rsx! {
        fieldset { class: "block border-0 p-0",
            legend { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Кто может писать" }
            div { class: "space-y-1.5",
                {mode_option(mode, ServerRoomWriteAccessMode::AllMembers, "Все участники", "Каждый участник сервера сможет писать в эту комнату.", on_mode_change)}
                {mode_option(mode, ServerRoomWriteAccessMode::SelectedRoles, "Только участники с выбранными ролями", "Писать будут владелец сервера и участники с этими ролями.", on_mode_change)}
            }
            div {
                class: "grid transition-[grid-template-rows,opacity] duration-200 ease-out motion-reduce:transition-none {roles_class}",
                "aria-hidden": if roles_selected_mode { "false" } else { "true" },
                div { class: "min-h-0 overflow-hidden",
                    fieldset { class: "min-w-0 border-0 px-0.5 pb-0.5 pt-3", disabled: !roles_selected_mode,
                        if roles.is_empty() {
                            p { class: "rounded-xl border border-zinc-800 bg-zinc-900/50 px-3 py-2.5 text-[12px] leading-5 text-zinc-500",
                                "На сервере пока нет ролей, которые можно выбрать. Создай роль в настройках сервера, чтобы ограничить запись."
                            }
                        } else {
                            div { class: "flex flex-wrap gap-2",
                                for role in roles {
                                    {role_chip(role, selected_role_ids.contains(&role.role_id), on_toggle_role)}
                                }
                            }
                            div {
                                class: "grid transition-[grid-template-rows,opacity] duration-200 ease-out motion-reduce:transition-none {warning_class}",
                                "aria-hidden": if warning_visible { "false" } else { "true" },
                                div { class: "min-h-0 overflow-hidden",
                                    div { class: "pt-3",
                                        p { class: "rounded-xl border border-amber-500/20 bg-amber-500/10 px-3 py-2.5 text-[12px] leading-5 text-amber-200",
                                            "Если не выбрать ни одной роли, писать в комнату сможет только владелец сервера."
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

/// Рендерит один переключатель режима доступа.
fn mode_option(
    current: ServerRoomWriteAccessMode,
    option: ServerRoomWriteAccessMode,
    title: &'static str,
    description: &'static str,
    on_mode_change: WriteAccessModeHandler,
) -> Element {
    let checked = current == option;
    let on_select = move |_| {
        debug!(mode = ?option, "changed room write access mode");
        on_mode_change.call(option);
    };

    rsx! {
        label { class: "flex cursor-pointer items-start gap-3 rounded-xl border border-zinc-800 bg-zinc-950/60 px-3 py-2.5 transition-[border-color,background] duration-150 hover:border-zinc-700",
            input {
                r#type: "radio",
                name: "room-write-mode",
                checked,
                onchange: on_select,
                class: "mt-0.5 size-4 shrink-0 accent-accent"
            }
            span { class: "min-w-0",
                span { class: "block text-[13px] font-medium text-zinc-200", "{title}" }
                span { class: "mt-0.5 block text-[12px] leading-5 text-zinc-500", "{description}" }
            }
        }
    }
}

/// Рендерит переключаемый чип роли.
fn role_chip(
    role: &ServerRoleSummary,
    selected: bool,
    on_toggle_role: WriteRoleToggleHandler,
) -> Element {
    let role_id = role.role_id.clone();
    let role_name = role.name.clone();
    let chip_class = if selected {
        "inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-[12px] font-medium ring-2 ring-accent/60 transition-[background,border-color,color,transform] duration-150"
    } else {
        "inline-flex items-center gap-1.5 rounded-full border border-zinc-800 bg-zinc-900/60 px-3 py-1.5 text-[12px] font-medium text-zinc-400 transition-[background,border-color,color,transform] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:text-zinc-200"
    };
    let on_toggle = move |_| {
        debug!(%role_id, selected = !selected, "toggled room write access role");
        on_toggle_role.call(role_id.clone());
    };

    rsx! {
        button {
            r#type: "button",
            class: chip_class,
            style: if selected { "background: {role.color}1f; border-color: {role.color}66; color: {role.color};" } else { "" },
            "aria-pressed": if selected { "true" } else { "false" },
            onclick: on_toggle,
            span { class: "grid size-3.5 shrink-0 place-items-center", "aria-hidden": "true",
                if selected {
                    svg {
                        class: "size-3.5 text-zinc-100",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2.5",
                        view_box: "0 0 24 24",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "m5 12 4 4L19 6" }
                    }
                } else {
                    span { class: "size-2 rounded-full opacity-60", style: "background: {role.color};" }
                }
            }
            "{role_name}"
        }
    }
}

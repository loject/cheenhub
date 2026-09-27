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

    rsx! {
        fieldset { class: "block border-0 p-0",
            legend { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Кто может писать" }
            div { class: "space-y-1.5",
                {mode_option(
                    mode,
                    ServerRoomWriteAccessMode::AllMembers,
                    "Все участники",
                    "Каждый участник сервера сможет писать в эту комнату.",
                    on_mode_change,
                )}
                {mode_option(
                    mode,
                    ServerRoomWriteAccessMode::SelectedRoles,
                    "Только участники с выбранными ролями",
                    "Писать будут владелец сервера и участники с этими ролями.",
                    on_mode_change,
                )}
            }
            if roles_selected_mode {
                if roles.is_empty() {
                    p { class: "mt-3 rounded-xl border border-zinc-800 bg-zinc-900/50 px-3 py-2.5 text-[12px] leading-5 text-zinc-500",
                        "На сервере пока нет ролей, которые можно выбрать. Создай роль в настройках сервера, чтобы ограничить запись."
                    }
                } else if selected_role_ids.is_empty() {
                    p { class: "mt-3 rounded-xl border border-amber-500/20 bg-amber-500/10 px-3 py-2.5 text-[12px] leading-5 text-amber-200",
                        "Если не выбрать ни одной роли, писать в комнату сможет только владелец сервера."
                    }
                } else {
                    div { class: "mt-3 flex flex-wrap gap-2",
                        for role in roles {
                            {
                                role_chip(
                                    role,
                                    selected_role_ids.contains(&role.role_id),
                                    on_toggle_role,
                                )
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
    let on_select = move |_| on_mode_change.call(option);

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
        "inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-[12px] font-medium transition-[background,border-color,color,transform] duration-150"
    } else {
        "inline-flex items-center gap-1.5 rounded-full border border-zinc-800 bg-zinc-900/60 px-3 py-1.5 text-[12px] font-medium text-zinc-400 transition-[background,border-color,color,transform] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:text-zinc-200"
    };
    let dot_class = if selected {
        "size-2 shrink-0 rounded-full"
    } else {
        "size-2 shrink-0 rounded-full opacity-60"
    };
    let on_toggle = move |_| on_toggle_role.call(role_id.clone());

    rsx! {
        button {
            r#type: "button",
            class: chip_class,
            style: if selected { "background: {role.color}1f; border-color: {role.color}66; color: {role.color};" } else { "" },
            "aria-pressed": if selected { "true" } else { "false" },
            onclick: on_toggle,
            span { class: dot_class, style: "background: {role.color};" }
            "{role_name}"
        }
    }
}

//! Role permissions card sub-component.

use dioxus::prelude::*;

use super::roles_data::{
    RoleDraft, RolePermission, permission_row_class, toggle_knob_class, toggle_track_class,
};

#[component]
pub(super) fn RolePermissionsCard(
    role: RoleDraft,
    perm_locked: bool,
    selected_role_owner: bool,
    is_owner: bool,
    on_permission_change: EventHandler<(RolePermission, bool)>,
) -> Element {
    let mut permission_search = use_signal(String::new);
    let visible_permissions = RolePermission::all()
        .iter()
        .copied()
        .filter(|permission| {
            let query = permission_search().trim().to_lowercase();
            query.is_empty()
                || permission.label().to_lowercase().contains(&query)
                || permission.hint().to_lowercase().contains(&query)
        })
        .collect::<Vec<_>>();

    rsx! {
        div { class: "role-editor-surface rounded-2xl border border-zinc-800 bg-zinc-950/70 p-5 transition-[border-color,background,box-shadow] duration-200 hover:border-zinc-700/80 hover:bg-zinc-950/80",
            div { class: "mb-5 flex flex-col gap-3 md:flex-row md:items-center md:justify-between",
                div {
                    p { class: "mb-2 inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/80 px-3 py-1 text-[10px] uppercase tracking-[0.24em] text-zinc-500",
                        span { class: "h-1.5 w-1.5 rounded-full bg-zinc-600" }
                        "Права"
                    }
                    h5 { class: "text-sm font-semibold text-zinc-50", "Права роли" }
                    p { class: "mt-1 text-[13px] leading-5 text-zinc-500",
                        if selected_role_owner {
                            "Владелец всегда имеет все права, поэтому переключатели заблокированы."
                        } else if !is_owner {
                            "Только владелец сервера может изменять права ролей."
                        } else {
                            "Включай только те права, которые нужны выбранной роли."
                        }
                    }
                }
                label { class: "flex h-10 min-w-0 items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900 px-3 text-[13px] text-zinc-500 transition-[background,border-color,box-shadow] duration-150 focus-within:border-zinc-700 focus-within:shadow-[0_0_0_4px_rgba(255,255,255,0.03)]",
                    svg { class: "h-4 w-4 shrink-0", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", d: "m21 21-4.3-4.3M11 19a8 8 0 1 1 0-16 8 8 0 0 1 0 16Z" }
                    }
                    input {
                        r#type: "search",
                        value: permission_search(),
                        placeholder: "Найти право",
                        oninput: move |event| permission_search.set(event.value()),
                        class: "w-40 bg-transparent text-zinc-200 outline-none placeholder:text-zinc-600",
                    }
                }
            }
            if visible_permissions.is_empty() {
                div { class: "rounded-2xl border border-zinc-800 bg-zinc-900/45 p-5 text-center text-[13px] text-zinc-500",
                    "Права с таким названием не найдены."
                }
            } else {
                div { class: "divide-y divide-zinc-800/50 rounded-2xl border border-zinc-800 bg-zinc-900/45 px-4",
                    for permission in visible_permissions {
                        label {
                            key: "{permission.key()}",
                            class: permission_row_class(perm_locked),
                            span { class: "min-w-0",
                                span { class: "block text-[13px] font-medium text-zinc-200", "{permission.label()}" }
                                span { class: "mt-0.5 block text-[12px] leading-5 text-zinc-500", "{permission.hint()}" }
                            }
                            span { class: "relative inline-flex shrink-0 items-center",
                                input {
                                    class: "peer sr-only",
                                    r#type: "checkbox",
                                    checked: role.has_permission(permission),
                                    disabled: perm_locked,
                                    onchange: move |event| {
                                        if perm_locked {
                                            return;
                                        }
                                        let checked = event.checked();
                                        on_permission_change((permission, checked));
                                    },
                                }
                                span { class: toggle_track_class(role.has_permission(permission), perm_locked),
                                    i { class: toggle_knob_class(role.has_permission(permission)) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

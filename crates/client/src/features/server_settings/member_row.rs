//! Member row component for the server settings members section.

use dioxus::prelude::*;

use super::members_data::ServerMemberRow;

#[component]
pub(super) fn MemberRow(
    member: ServerMemberRow,
    custom_roles: Vec<super::members_data::CustomRole>,
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

//! Server invite list item component.

use dioxus::prelude::*;

use super::invites_data::{InviteLink, InviteStatus};

/// User intent emitted by an invite list item.
#[derive(Clone, PartialEq)]
pub(super) enum InviteListItemAction {
    /// Copy the invite link.
    CopyInvite {
        /// Invite code shown in the UI.
        invite_code: String,
    },
    /// Remove the invite.
    RemoveInvite {
        /// Stable invite row id.
        invite_id: String,
        /// Invite code shown in the UI.
        invite_code: String,
    },
    /// Toggle usage details for the invite.
    ToggleUsage {
        /// Stable invite row id.
        invite_id: String,
        /// Invite code shown in the UI.
        invite_code: String,
    },
    /// Open a context menu for a joined member.
    OpenMemberMenu {
        /// Stable invite row id.
        invite_id: String,
        /// Invite code shown in the UI.
        invite_code: String,
        /// Stable member id.
        member_id: String,
        /// Member display name.
        member_name: String,
        /// Menu x coordinate.
        x: f64,
        /// Menu y coordinate.
        y: f64,
    },
}

/// Renders one invite link row in server settings.
#[component]
pub(super) fn InviteListItem(
    invite: InviteLink,
    usage_expanded: bool,
    on_action: EventHandler<InviteListItemAction>,
) -> Element {
    rsx! {
        div {
            class: "rounded-2xl border border-zinc-800 bg-zinc-900/45 p-3",
            div { class: "flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between",
                div { class: "min-w-0 flex items-start gap-3",
                    span { class: invite_icon_class(invite.status),
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M13.19 8.688a4.5 4.5 0 0 1 1.242 7.244l-4.5 4.5a4.5 4.5 0 0 1-6.364-6.364l1.757-1.757m13.35-.622 1.757-1.757a4.5 4.5 0 0 0-6.364-6.364l-4.5 4.5a4.5 4.5 0 0 0 1.242 7.244" }
                        }
                    }
                    div { class: "min-w-0",
                        div { class: "flex flex-wrap items-center gap-2",
                            p { class: "break-all font-mono text-[13px] font-semibold text-zinc-100", "{invite.code}" }
                            button {
                                r#type: "button",
                                class: "flex h-7 w-7 shrink-0 items-center justify-center rounded-lg border border-zinc-800 bg-zinc-950 text-zinc-500 transition hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-200",
                                "aria-label": "Скопировать ссылку приглашения",
                                onclick: {
                                    let invite_code = invite.code.clone();
                                    move |event| {
                                        event.stop_propagation();
                                        on_action.call(InviteListItemAction::CopyInvite {
                                            invite_code: invite_code.clone(),
                                        });
                                    }
                                },
                                svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                    rect { x: "8", y: "8", width: "11", height: "11", rx: "2", ry: "2" }
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" }
                                }
                            }
                            span { class: status_badge_class(invite.status), "{status_label(invite.status)}" }
                        }
                        p { class: "mt-1 text-[11px] leading-4 text-zinc-500",
                            "Создал {invite.author} · {invite.created_at} · действует {invite.expires_at}"
                        }
                    }
                }
                div { class: "flex shrink-0 flex-wrap gap-2",
                    button {
                        r#type: "button",
                        disabled: invite.status == InviteStatus::Revoked,
                        class: revoke_button_class(invite.status),
                        onclick: {
                            let invite_id = invite.id.clone();
                            let invite_code = invite.code.clone();
                            let invite_status = invite.status;
                            move |_| {
                                if invite_status == InviteStatus::Revoked {
                                    return;
                                }
                                on_action.call(InviteListItemAction::RemoveInvite {
                                    invite_id: invite_id.clone(),
                                    invite_code: invite_code.clone(),
                                });
                            }
                        },
                        "{revoke_button_label(invite.status)}"
                    }
                }
            }
            div { class: "mt-3 grid gap-2 sm:grid-cols-3",
                button {
                    r#type: "button",
                    class: usage_metric_class(usage_expanded),
                    "aria-expanded": if usage_expanded { "true" } else { "false" },
                    onclick: {
                        let invite_id = invite.id.clone();
                        let invite_code = invite.code.clone();
                        move |_| on_action.call(InviteListItemAction::ToggleUsage {
                            invite_id: invite_id.clone(),
                            invite_code: invite_code.clone(),
                        })
                    },
                    p { class: "text-[10px] font-medium uppercase tracking-[0.14em] text-zinc-600", "Использования" }
                    p { class: "mt-1 text-[12px] font-medium text-zinc-200", "{usage_text(invite.joined_members.len() as u32, invite.max_uses)}" }
                }
                {invite_metric("Осталось", remaining_text(invite.joined_members.len() as u32, invite.max_uses))}
                {invite_metric("Доступ", access_text(invite.status))}
            }
            if usage_expanded {
                div { class: "mt-3 rounded-2xl border border-zinc-800 bg-zinc-950/70 p-3",
                    div { class: "flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between",
                        p { class: "text-[13px] font-semibold text-zinc-100", "Кто присоединился" }
                        p { class: "text-[11px] text-zinc-500", "{invite.joined_members.len()} входа по этой ссылке" }
                    }
                    if invite.joined_members.is_empty() {
                        div { class: "mt-3 rounded-xl border border-zinc-800 bg-zinc-900/45 px-3 py-2",
                            p { class: "text-[12px] font-medium text-zinc-200", "По этой ссылке еще никто не присоединился" }
                            p { class: "mt-1 text-[11px] leading-4 text-zinc-500", "Новые участники появятся здесь после первого входа." }
                        }
                    } else {
                        div { class: "mt-3 max-h-64 space-y-2 overflow-y-auto pr-1",
                            for member in invite.joined_members.iter() {
                                div {
                                    key: "{member.id}",
                                    class: "flex items-center justify-between gap-3 rounded-xl border border-zinc-800 bg-zinc-900/45 px-3 py-2",
                                    oncontextmenu: {
                                        let invite_id = invite.id.clone();
                                        let invite_code = invite.code.clone();
                                        let member_id = member.id.clone();
                                        let member_name = member.name.clone();
                                        let is_active_member = member.is_active_member;
                                        move |event| {
                                            if !is_active_member {
                                                return;
                                            }
                                            event.prevent_default();
                                            event.stop_propagation();
                                            let point = event.client_coordinates();
                                            on_action.call(InviteListItemAction::OpenMemberMenu {
                                                invite_id: invite_id.clone(),
                                                invite_code: invite_code.clone(),
                                                member_id: member_id.clone(),
                                                member_name: member_name.clone(),
                                                x: point.x,
                                                y: point.y,
                                            });
                                        }
                                    },
                                    div { class: "min-w-0",
                                        p { class: "truncate text-[12px] font-medium text-zinc-100", "{member.name}" }
                                        p { class: "mt-0.5 truncate font-mono text-[10px] text-zinc-600", "{member.id}" }
                                    }
                                    div { class: "flex shrink-0 items-center gap-2",
                                        p { class: "text-right text-[11px] text-zinc-500", "{member.joined_at}" }
                                        if member.is_active_member {
                                            button {
                                                r#type: "button",
                                                class: "flex h-8 w-8 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-zinc-500 transition hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-200",
                                                "aria-label": "Меню участника",
                                                onclick: {
                                                    let invite_id = invite.id.clone();
                                                    let invite_code = invite.code.clone();
                                                    let member_id = member.id.clone();
                                                    let member_name = member.name.clone();
                                                    move |event| {
                                                        event.stop_propagation();
                                                        let point = event.client_coordinates();
                                                        on_action.call(InviteListItemAction::OpenMemberMenu {
                                                            invite_id: invite_id.clone(),
                                                            invite_code: invite_code.clone(),
                                                            member_id: member_id.clone(),
                                                            member_name: member_name.clone(),
                                                            x: point.x,
                                                            y: point.y,
                                                        });
                                                    }
                                                },
                                                svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6.75 12a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm6 0a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Z" }
                                                }
                                            }
                                        } else {
                                            span { class: "rounded-full border border-zinc-800 bg-zinc-950 px-2 py-1 text-[10px] font-medium text-zinc-500", "Исключен" }
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

fn invite_metric(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-950/70 px-3 py-2",
            p { class: "text-[10px] font-medium uppercase tracking-[0.14em] text-zinc-600", "{label}" }
            p { class: "mt-1 text-[12px] font-medium text-zinc-200", "{value}" }
        }
    }
}

fn usage_metric_class(expanded: bool) -> &'static str {
    if expanded {
        "rounded-xl border border-accent/30 bg-accent/10 px-3 py-2 text-left transition hover:border-accent/45 hover:bg-accent/15"
    } else {
        "rounded-xl border border-zinc-800 bg-zinc-950/70 px-3 py-2 text-left transition hover:border-accent/30 hover:bg-accent/10"
    }
}

fn invite_icon_class(status: InviteStatus) -> &'static str {
    match status {
        InviteStatus::Active => {
            "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-accent/25 bg-accent/10 text-blue-200"
        }
        InviteStatus::Revoked => {
            "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-zinc-500"
        }
    }
}

fn status_badge_class(status: InviteStatus) -> &'static str {
    match status {
        InviteStatus::Active => {
            "rounded-full border border-emerald-500/20 bg-emerald-500/10 px-2 py-0.5 text-[10px] font-medium text-emerald-200"
        }
        InviteStatus::Revoked => {
            "rounded-full border border-zinc-700 bg-zinc-950 px-2 py-0.5 text-[10px] font-medium text-zinc-400"
        }
    }
}

fn status_label(status: InviteStatus) -> &'static str {
    match status {
        InviteStatus::Active => "Активна",
        InviteStatus::Revoked => "Отозвана",
    }
}

fn revoke_button_class(status: InviteStatus) -> &'static str {
    match status {
        InviteStatus::Active => {
            "flex h-9 items-center justify-center rounded-xl border border-red-500/25 bg-red-500/10 px-3 text-[12px] font-medium text-red-200 transition hover:border-red-500/35 hover:bg-red-500/15"
        }
        InviteStatus::Revoked => {
            "flex h-9 cursor-not-allowed items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-medium text-zinc-500"
        }
    }
}

fn revoke_button_label(status: InviteStatus) -> &'static str {
    match status {
        InviteStatus::Active => "Отозвать",
        InviteStatus::Revoked => "Восстановить",
    }
}

fn usage_text(uses: u32, max_uses: Option<u32>) -> String {
    match max_uses {
        Some(limit) => format!("{uses} из {limit}"),
        None => format!("{uses}, без лимита"),
    }
}

fn remaining_text(uses: u32, max_uses: Option<u32>) -> String {
    match max_uses {
        Some(limit) => limit.saturating_sub(uses).to_string(),
        None => "без лимита".to_owned(),
    }
}

fn access_text(status: InviteStatus) -> String {
    match status {
        InviteStatus::Active => "доступна для входа".to_owned(),
        InviteStatus::Revoked => "отозвана".to_owned(),
    }
}

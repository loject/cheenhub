//! Server invite settings section.

use dioxus::prelude::*;

use super::invite_list_item::{InviteListItem, InviteListItemAction};
use super::invites_data::{InviteStatus, mock_invites};

#[derive(Clone, PartialEq)]
struct MemberMenuState {
    invite_id: String,
    invite_code: String,
    member_id: String,
    member_name: String,
    x: f64,
    y: f64,
}

/// Renders server invite-link viewing and management controls.
#[component]
pub(crate) fn ServerInvitesSettingsSection(server_name: String) -> Element {
    let mut invites = use_signal(mock_invites);
    let mut only_active = use_signal(|| true);
    let mut status_message = use_signal(String::new);
    let mut expanded_usage_invite = use_signal(|| None::<String>);
    let mut open_member_menu = use_signal(|| None::<MemberMenuState>);
    let all_invites = invites();
    let active_count = all_invites
        .iter()
        .filter(|invite| invite.status == InviteStatus::Active)
        .count();
    let paused_count = all_invites.len().saturating_sub(active_count);
    let visible_invites = all_invites
        .iter()
        .filter(|invite| !only_active() || invite.status == InviteStatus::Active)
        .cloned()
        .collect::<Vec<_>>();

    rsx! {
        div {
            class: "space-y-4",
            onclick: move |_| open_member_menu.set(None),
            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                div { class: "flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between",
                    div { class: "min-w-0",
                        h3 { class: "text-[18px] font-semibold tracking-[-0.03em] text-zinc-50", "Инвайт-ссылки" }
                        p { class: "mt-1 max-w-2xl text-[12px] leading-5 text-zinc-500",
                            "Ссылки для входа на сервер {server_name}. Управляй сроком действия, лимитами и доступностью приглашений."
                        }
                    }
                    div { class: "grid grid-cols-2 gap-2 sm:flex sm:shrink-0",
                        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/70 px-3 py-2",
                            p { class: "text-[11px] text-zinc-500", "Активные" }
                            p { class: "mt-0.5 text-[16px] font-semibold text-zinc-100", "{active_count}" }
                        }
                        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/70 px-3 py-2",
                            p { class: "text-[11px] text-zinc-500", "Приостановлены" }
                            p { class: "mt-0.5 text-[16px] font-semibold text-zinc-100", "{paused_count}" }
                        }
                    }
                }
            }

            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                div { class: "flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between",
                    div {
                        h4 { class: "text-[15px] font-semibold text-zinc-50", "Действующие приглашения" }
                        p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Отслеживай использование и отключай ссылки, которые больше не нужны." }
                    }
                    label { class: "flex min-h-9 cursor-pointer items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900/70 px-3 text-[12px] font-medium text-zinc-300",
                        input {
                            r#type: "checkbox",
                            checked: only_active(),
                            onchange: move |event| only_active.set(event.checked()),
                            class: "h-4 w-4 rounded bg-zinc-950 accent-blue-500"
                        }
                        span { "Только активные" }
                    }
                }

                if visible_invites.is_empty() {
                    div { class: "mt-4 rounded-2xl border border-zinc-800 bg-zinc-900/45 p-5",
                        div { class: "flex max-w-lg flex-col gap-3",
                            span { class: "flex h-10 w-10 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-zinc-500",
                                svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10.5 6H6.75A2.25 2.25 0 0 0 4.5 8.25v9A2.25 2.25 0 0 0 6.75 19.5h10.5A2.25 2.25 0 0 0 19.5 17.25V13.5m-3-9h3m0 0v3m0-3-7.5 7.5" }
                                }
                            }
                            div {
                                p { class: "text-[13px] font-medium text-zinc-100", "Подходящих ссылок нет" }
                                p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Сними фильтр, чтобы увидеть остальные приглашения." }
                            }
                        }
                    }
                } else {
                    div { class: "mt-4 space-y-2",
                        for invite in visible_invites {
                            InviteListItem {
                                key: "{invite.id}",
                                invite: invite.clone(),
                                usage_expanded: expanded_usage_invite().as_deref() == Some(invite.id.as_str()),
                                on_action: move |action| {
                                    match action {
                                        InviteListItemAction::CopyLink { invite_code } => {
                                            status_message.set(format!("Ссылка {invite_code} готова для копирования."));
                                            info!(
                                                invite_code = %invite_code,
                                                "selected server invite link for copying in settings ui"
                                            );
                                        }
                                        InviteListItemAction::ToggleStatus { invite_id, invite_code } => {
                                            let next_status = invites()
                                                .into_iter()
                                                .find(|existing| existing.id == invite_id)
                                                .map(|existing| {
                                                    if existing.status == InviteStatus::Active {
                                                        InviteStatus::Paused
                                                    } else {
                                                        InviteStatus::Active
                                                    }
                                                })
                                                .unwrap_or(InviteStatus::Active);
                                            invites.set(
                                                invites()
                                                    .into_iter()
                                                    .map(|mut existing| {
                                                        if existing.id == invite_id {
                                                            existing.status = next_status;
                                                        }
                                                        existing
                                                    })
                                                    .collect::<Vec<_>>(),
                                            );
                                            status_message.set(format!(
                                                "Статус ссылки {invite_code}: {}.",
                                                status_label(next_status)
                                            ));
                                            info!(
                                                invite_code = %invite_code,
                                                status = status_label(next_status),
                                                "changed local server invite link status in settings ui"
                                            );
                                        }
                                        InviteListItemAction::RemoveInvite { invite_id, invite_code } => {
                                            invites.set(
                                                invites()
                                                    .into_iter()
                                                    .filter(|existing| existing.id != invite_id)
                                                    .collect::<Vec<_>>(),
                                            );
                                            status_message.set(format!("Ссылка {invite_code} отозвана."));
                                            info!(
                                                invite_code = %invite_code,
                                                "removed local server invite link in settings ui"
                                            );
                                        }
                                        InviteListItemAction::ToggleUsage { invite_id, invite_code } => {
                                            if expanded_usage_invite().as_deref() == Some(invite_id.as_str()) {
                                                expanded_usage_invite.set(None);
                                                info!(
                                                    invite_code = %invite_code,
                                                    "collapsed server invite usage details in settings ui"
                                                );
                                            } else {
                                                expanded_usage_invite.set(Some(invite_id));
                                                info!(
                                                    invite_code = %invite_code,
                                                    "expanded server invite usage details in settings ui"
                                                );
                                            }
                                        }
                                        InviteListItemAction::OpenMemberMenu {
                                            invite_id,
                                            invite_code,
                                            member_id,
                                            member_name,
                                            x,
                                            y,
                                        } => {
                                            open_member_menu.set(Some(MemberMenuState {
                                                invite_id,
                                                invite_code,
                                                member_id,
                                                member_name,
                                                x,
                                                y,
                                            }));
                                        }
                                    }
                                },
                            }
                        }
                    }
                }

                if !status_message().is_empty() {
                    p { class: "mt-4 rounded-xl border border-zinc-800 bg-zinc-900/80 px-3 py-2 text-[12px] leading-5 text-zinc-300",
                        "{status_message()}"
                    }
                }
            }

            if let Some(menu) = open_member_menu() {
                div {
                    class: "fixed inset-0 z-[999] cursor-default",
                    "aria-label": "Закрыть меню участника",
                    onclick: move |_| open_member_menu.set(None),
                }
                div {
                    class: "fixed z-[1000] w-[246px] rounded-[20px] border border-zinc-800 bg-zinc-950/95 p-2 shadow-[0_22px_70px_rgba(0,0,0,.60)] backdrop-blur-xl",
                    style: member_menu_style(menu.x, menu.y),
                    onclick: move |event| event.stop_propagation(),
                    div { class: "px-2 pb-2 pt-1",
                        div { class: "truncate text-[12px] font-medium text-zinc-200", "{menu.member_name}" }
                        div { class: "mt-0.5 truncate font-mono text-[10px] text-zinc-600", "{menu.member_id}" }
                    }
                    div { class: "my-1 border-t border-zinc-800" }
                    button {
                        r#type: "button",
                        class: "flex w-full items-center justify-between rounded-xl px-3 py-2.5 text-left text-[13px] text-red-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:bg-red-500/10 hover:text-red-200",
                        onclick: {
                            let invite_id = menu.invite_id.clone();
                            let invite_code = menu.invite_code.clone();
                            let member_id = menu.member_id.clone();
                            let member_name = menu.member_name.clone();
                            move |_| {
                                invites.set(
                                    invites()
                                        .into_iter()
                                        .map(|mut invite| {
                                            if invite.id == invite_id {
                                                invite
                                                    .joined_members
                                                    .retain(|member| member.id != member_id.as_str());
                                            }
                                            invite
                                        })
                                        .collect::<Vec<_>>(),
                                );
                                status_message.set(format!("{member_name} исключен с сервера."));
                                open_member_menu.set(None);
                                info!(
                                    invite_code = %invite_code,
                                    member_id = %member_id,
                                    member_name = %member_name,
                                    "kicked server invite member in settings ui"
                                );
                            }
                        },
                        span { "Кикнуть с сервера" }
                    }
                }
            }
        }
    }
}

fn member_menu_style(x: f64, y: f64) -> String {
    let top = y + 8.0;
    format!(
        "left: clamp(12px, {x}px, calc(100vw - 258px)); top: clamp(12px, {top}px, calc(100vh - 178px));"
    )
}

fn status_label(status: InviteStatus) -> &'static str {
    match status {
        InviteStatus::Active => "Активна",
        InviteStatus::Paused => "Приостановлена",
    }
}

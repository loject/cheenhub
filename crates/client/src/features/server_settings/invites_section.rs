//! Server invite settings section.

use dioxus::prelude::*;

use super::invite_list_item::{InviteListItem, InviteListItemAction};
use super::invites_data::{InviteLink, InviteStatus, invite_from_realtime};
use super::realtime;
use crate::features::realtime::RealtimeHandle;
use crate::features::toast::ToastHandle;

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
pub(crate) fn ServerInvitesSettingsSection(server_id: String, server_name: String) -> Element {
    let realtime_handle = use_context::<RealtimeHandle>();
    let toast = use_context::<ToastHandle>();
    let mut invites = use_signal(|| None::<Vec<InviteLink>>);
    let mut only_active = use_signal(|| true);
    let mut load_error = use_signal(String::new);
    let mut pending_action = use_signal(|| None::<String>);
    let mut refresh_requested = use_signal(|| false);
    let mut expanded_usage_invite = use_signal(|| None::<String>);
    let mut open_member_menu = use_signal(|| None::<MemberMenuState>);
    let load_server_id = server_id.clone();
    let load_realtime_handle = realtime_handle.clone();
    let mut invite_load = use_resource(move || {
        let realtime_handle = load_realtime_handle.clone();
        let request_server_id = load_server_id.clone();

        async move { realtime::list_server_invites(&realtime_handle, request_server_id).await }
    });
    let invite_load_result = invite_load.read().clone();
    use_effect(move || {
        if invites().is_some() {
            return;
        }

        let Some(result) = invite_load.read().clone() else {
            return;
        };

        match result {
            Ok(response) => {
                invites.set(Some(
                    response
                        .invites
                        .into_iter()
                        .map(invite_from_realtime)
                        .collect(),
                ));
                if refresh_requested() {
                    toast.success("Список приглашений обновлен.");
                    info!(
                        server_id = %response.server_id,
                        "refreshed server invite links in settings ui"
                    );
                }
                refresh_requested.set(false);
                load_error.set(String::new());
            }
            Err(error) => {
                if refresh_requested() {
                    toast.error(error.to_string());
                }
                load_error.set(error.to_string());
                refresh_requested.set(false);
            }
        }
    });

    let all_invites = invites().unwrap_or_default();
    let is_loading = invites().is_none() && invite_load_result.is_none();
    let active_count = all_invites
        .iter()
        .filter(|invite| invite.status == InviteStatus::Active)
        .count();
    let revoked_count = all_invites.len().saturating_sub(active_count);
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
                            p { class: "text-[11px] text-zinc-500", "Отозваны" }
                            p { class: "mt-0.5 text-[16px] font-semibold text-zinc-100", "{revoked_count}" }
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
                    div { class: "flex flex-wrap items-center gap-2",
                        button {
                            r#type: "button",
                            disabled: is_loading || pending_action().is_some(),
                            class: refresh_button_class(is_loading || pending_action().is_some()),
                            onclick: move |_| {
                                if is_loading || pending_action().is_some() {
                                    return;
                                }

                                load_error.set(String::new());
                                refresh_requested.set(true);
                                invites.set(None);
                                open_member_menu.set(None);
                                invite_load.clear();
                                invite_load.restart();
                                info!("refreshing server invite links in settings ui");
                            },
                            svg { class: if is_loading { "h-4 w-4 animate-spin" } else { "h-4 w-4" }, fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99" }
                            }
                            "Обновить"
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
                }

                if is_loading {
                    div { class: "mt-4 space-y-2",
                        div { class: "h-[104px] animate-pulse rounded-2xl border border-zinc-800 bg-zinc-900/55" }
                        div { class: "h-[104px] animate-pulse rounded-2xl border border-zinc-800 bg-zinc-900/40" }
                    }
                } else if !load_error().is_empty() && invites().is_none() {
                    div { class: "mt-4 rounded-2xl border border-red-500/20 bg-red-500/10 p-4",
                        p { class: "text-[13px] font-medium text-red-100", "Не удалось загрузить приглашения" }
                        p { class: "mt-1 text-[12px] leading-5 text-red-200", "{load_error()}" }
                    }
                } else if visible_invites.is_empty() {
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
                                on_action: {
                                    let action_realtime_handle = realtime_handle.clone();
                                    let action_server_id = server_id.clone();
                                    move |action| {
                                    match action {
                                        InviteListItemAction::CopyInvite { invite_code } => {
                                            match clipboard_copy_invite_link(invite_code.clone()) {
                                                Ok(copy) => {
                                                    spawn(async move {
                                                        match copy.await {
                                                            Ok(()) => {
                                                                toast.success("Ссылка приглашения скопирована.");
                                                                info!(
                                                                    invite_code = %invite_code,
                                                                    "copied server invite link in settings ui"
                                                                );
                                                            }
                                                            Err(error) => {
                                                                toast.error(error.clone());
                                                                warn!(
                                                                    %error,
                                                                    invite_code = %invite_code,
                                                                    "failed to copy server invite link in settings ui"
                                                                );
                                                            }
                                                        }
                                                    });
                                                }
                                                Err(error) => {
                                                    toast.error(error.clone());
                                                    warn!(
                                                        %error,
                                                        invite_code = %invite_code,
                                                        "failed to prepare server invite link copying in settings ui"
                                                    );
                                                }
                                            }
                                        }
                                        InviteListItemAction::RemoveInvite { invite_id, invite_code } => {
                                            if pending_action().is_some() {
                                                return;
                                            }

                                            pending_action.set(Some(format!("Отзываем ссылку {invite_code}...")));
                                            let action_realtime = action_realtime_handle.clone();
                                            let action_server_id = action_server_id.clone();
                                            let revoked_invite_id = invite_id.clone();
                                            let revoked_invite_code = invite_code.clone();
                                            spawn(async move {
                                                match realtime::revoke_server_invite(
                                                    &action_realtime,
                                                    action_server_id,
                                                    revoked_invite_code.clone(),
                                                )
                                                .await
                                                {
                                                    Ok(response) => {
                                                        invites.set(Some(
                                                            invites()
                                                                .unwrap_or_default()
                                                                .into_iter()
                                                                .filter(|existing| existing.id != revoked_invite_id)
                                                                .collect::<Vec<_>>(),
                                                        ));
                                                        pending_action.set(None);
                                                        toast.success("Ссылка приглашения отозвана.");
                                                        info!(
                                                            invite_code = %response.code,
                                                            "revoked server invite link in settings ui"
                                                        );
                                                    }
                                                    Err(error) => {
                                                        pending_action.set(None);
                                                        toast.error(error.to_string());
                                                        warn!(
                                                            %error,
                                                            "failed to revoke server invite link in settings ui"
                                                        );
                                                    }
                                                }
                                            });
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
                                    }
                                },
                            }
                        }
                    }
                }

                if let Some(action) = pending_action() {
                    p { class: "mt-4 rounded-xl border border-accent/20 bg-accent/10 px-3 py-2 text-[12px] leading-5 text-blue-100",
                        "{action}"
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
                            let action_realtime_handle = realtime_handle.clone();
                            let action_server_id = server_id.clone();
                            let invite_id = menu.invite_id.clone();
                            let invite_code = menu.invite_code.clone();
                            let member_id = menu.member_id.clone();
                            let member_name = menu.member_name.clone();
                            move |_| {
                                if pending_action().is_some() {
                                    return;
                                }

                                pending_action.set(Some(format!("Исключаем {member_name}...")));
                                open_member_menu.set(None);
                                let action_realtime = action_realtime_handle.clone();
                                let action_server_id = action_server_id.clone();
                                let kicked_invite_id = invite_id.clone();
                                let kicked_invite_code = invite_code.clone();
                                let kicked_member_id = member_id.clone();
                                let kicked_member_name = member_name.clone();
                                spawn(async move {
                                    match realtime::kick_server_invite_member(
                                        &action_realtime,
                                        action_server_id,
                                        kicked_invite_code.clone(),
                                        kicked_member_id.clone(),
                                    )
                                    .await
                                    {
                                        Ok(response) => {
                                            invites.set(Some(
                                                invites()
                                                    .unwrap_or_default()
                                                    .into_iter()
                                                    .map(|mut invite| {
                                                        if invite.id == kicked_invite_id {
                                                            for member in &mut invite.joined_members {
                                                                if member.id == response.user_id {
                                                                    member.is_active_member = false;
                                                                }
                                                            }
                                                        }
                                                        invite
                                                    })
                                                    .collect::<Vec<_>>(),
                                            ));
                                            pending_action.set(None);
                                            toast.warning(format!("{kicked_member_name} исключен с сервера."));
                                            info!(
                                                invite_code = %response.invite_code,
                                                member_id = %response.user_id,
                                                member_name = %kicked_member_name,
                                                "kicked server invite member in settings ui"
                                            );
                                        }
                                        Err(error) => {
                                            pending_action.set(None);
                                            toast.error(error.to_string());
                                            warn!(
                                                %error,
                                                invite_code = %kicked_invite_code,
                                                member_id = %kicked_member_id,
                                                "failed to kick server invite member in settings ui"
                                            );
                                        }
                                    }
                                });
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

fn refresh_button_class(disabled: bool) -> &'static str {
    if disabled {
        "flex min-h-9 cursor-wait items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900/50 px-3 text-[12px] font-medium text-zinc-500"
    } else {
        "flex min-h-9 items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900/70 px-3 text-[12px] font-medium text-zinc-300 transition hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100"
    }
}

fn clipboard_copy_invite_link(
    code: String,
) -> Result<impl std::future::Future<Output = Result<(), String>>, String> {
    let compact_code = code.replace('-', "");
    let eval = document::eval(
        r#"
        const compactCode = await dioxus.recv();
        const origin = window.location.origin.replace(/\/$/, "");
        await navigator.clipboard.writeText(`${origin}/invite/${compactCode}`);
        return true;
        "#,
    );
    eval.send(compact_code)
        .map_err(|_| "Не удалось подготовить копирование.".to_owned())?;

    Ok(async move {
        eval.join::<bool>()
            .await
            .map(|_| ())
            .map_err(|_| "Браузер не разрешил скопировать ссылку.".to_owned())
    })
}

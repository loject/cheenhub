//! Уведомление о переписке с пользователем вне списка друзей.

use cheenhub_contracts::rest::{DmBlockStatus, UserRelationStatus};

use crate::features::privacy::api as privacy_api;
use dioxus::prelude::*;

use super::api;

#[cfg(test)]
mod tests;

/// Показывает ограничения диалога или предложение дружбы.
///
/// При собственной блокировке даёт снять её, сохраняя историю переписки.
#[component]
pub(super) fn ConversationNotice(
    user_id: String,
    relation: Option<UserRelationStatus>,
    block_status: DmBlockStatus,
    incoming_request_id: Option<String>,
    on_changed: EventHandler<()>,
) -> Element {
    let mut pending = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut completed = use_signal(|| None::<UserRelationStatus>);
    use_effect(use_reactive!(|relation| {
        let _ = relation;
        completed.set(None);
    }));
    let displayed_relation = completed().or(relation);
    if block_status != DmBlockStatus::None {
        let can_unblock = matches!(
            block_status,
            DmBlockStatus::BlockedByMe | DmBlockStatus::Mutual
        );
        let (heading, description) = match block_status {
            DmBlockStatus::BlockedByMe => (
                "Вы заблокировали этого пользователя",
                "Сообщения, звонки и приглашения недоступны. История переписки сохранена.",
            ),
            DmBlockStatus::BlockedByPeer => (
                "Пользователь вас заблокировал",
                "Вы не можете отправлять сообщения, звонить или приглашать в голосовую комнату.",
            ),
            DmBlockStatus::Mutual => (
                "Вы заблокировали друг друга",
                "Личные контакты недоступны. Вы можете снять свою блокировку.",
            ),
            DmBlockStatus::None => unreachable!(),
        };
        return rsx! {
            aside { class: "shrink-0 border-b border-red-500/15 bg-red-500/[0.06] px-5 py-3", role: "status",
                div { class: "flex flex-wrap items-center justify-between gap-3",
                    div {
                        p { class: "text-[13px] font-semibold text-red-200", "{heading}" }
                        p { class: "mt-1 text-[12px] leading-5 text-zinc-400", "{description}" }
                    }
                    if can_unblock {
                        button {
                            r#type: "button",
                            disabled: pending(),
                            class: "min-h-10 shrink-0 rounded-xl border border-red-500/25 bg-red-500/10 px-3 text-[12px] font-medium text-red-100 transition hover:bg-red-500/20 disabled:opacity-50",
                            onclick: move |_| {
                                if pending() { return; }
                                pending.set(true);
                                error.set(None);
                                let peer_user_id = user_id.clone();
                                spawn(async move {
                                    info!(%peer_user_id, "unblocking user from direct dialog");
                                    match privacy_api::unblock_user(&peer_user_id).await {
                                        Ok(()) => {
                                            info!(%peer_user_id, "user unblocked from direct dialog");
                                            on_changed.call(());
                                        }
                                        Err(cause) => {
                                            warn!(%peer_user_id, %cause, "failed to unblock user from direct dialog");
                                            error.set(Some(cause));
                                        }
                                    }
                                    pending.set(false);
                                });
                            },
                            if pending() { "Разблокируем…" } else { "Разблокировать" }
                        }
                    }
                }
                if let Some(message) = error() {
                    p { role: "alert", class: "mt-2 text-[12px] text-red-200", "{message}" }
                }
            }
        };
    }
    if displayed_relation == Some(UserRelationStatus::Friends) {
        return rsx! {};
    }
    let incoming = displayed_relation == Some(UserRelationStatus::PendingIncoming);
    let outgoing = displayed_relation == Some(UserRelationStatus::PendingOutgoing);
    let label = if pending() {
        "Подождите…"
    } else if outgoing {
        "Заявка отправлена"
    } else if incoming {
        "Принять заявку"
    } else {
        "Добавить в друзья"
    };

    rsx! {
        aside { class: "shrink-0 bg-amber-500/[0.06] px-5 py-3 shadow-[inset_0_-1px_0_rgba(245,158,11,0.12)]", "aria-label": "Собеседник вне списка друзей",
            div { class: "flex flex-wrap items-center justify-between gap-3",
                p { class: "text-[13px] leading-5 text-amber-100/90", "Этот пользователь не у вас в друзьях" }
                div { class: "flex flex-wrap items-center gap-2",
                    button {
                        r#type: "button",
                        disabled: pending() || outgoing || incoming && incoming_request_id.is_none(),
                        class: "flex min-h-10 items-center gap-2 rounded-xl bg-zinc-800/80 px-3 text-[12px] font-medium text-zinc-100 transition-[background-color,opacity] duration-150 hover:bg-zinc-700 disabled:opacity-60",
                        onclick: move |_| {
                            if *pending.peek() || outgoing { return; }
                            pending.set(true);
                            error.set(None);
                            let user_id = user_id.clone();
                            let request_id = incoming_request_id.clone();
                            spawn(async move {
                                info!(peer_user_id = %user_id, incoming, "updating friendship from direct dialog");
                                let result = if incoming {
                                    match request_id {
                                        Some(id) => api::accept_friend_request(&id).await,
                                        None => Err("Не удалось найти заявку. Попробуйте открыть диалог снова.".to_owned()),
                                    }
                                } else { api::send_friend_request(user_id.clone()).await };
                                pending.set(false);
                                match result {
                                    Ok(response) => {
                                        completed.set(Some(if response.request.status == cheenhub_contracts::rest::FriendRequestStatus::Accepted { UserRelationStatus::Friends } else { UserRelationStatus::PendingOutgoing }));
                                        on_changed.call(());
                                    }
                                    Err(message) => {
                                        warn!(peer_user_id = %user_id, %message, "failed to update friendship from direct dialog");
                                        error.set(Some(message));
                                        on_changed.call(());
                                    }
                                }
                            });
                        },
                        if pending() { span { class: "h-3 w-3 animate-spin rounded-full border-2 border-current border-t-transparent", "aria-hidden": "true" } }
                        "{label}"
                    }
                }
            }
            if let Some(message) = error() { p { role: "alert", class: "mt-2 text-[12px] text-red-200", "{message}" } }
        }
    }
}

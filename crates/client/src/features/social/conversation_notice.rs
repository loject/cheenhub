//! Уведомление о переписке с пользователем вне списка друзей.

use cheenhub_contracts::rest::UserRelationStatus;
use dioxus::prelude::*;

use super::api;

#[cfg(test)]
mod tests;

/// Предлагает отправить или принять заявку в друзья, сохраняя возможность переписки.
///
/// Блокировка пока недоступна и явно обозначена подсказкой при наведении и фокусе.
#[component]
pub(super) fn ConversationNotice(
    user_id: String,
    relation: Option<UserRelationStatus>,
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
                    span { class: "group relative", title: "Блокировка в разработке",
                        button { r#type: "button", "aria-disabled": "true", "aria-describedby": "dm-block-unavailable", class: "min-h-10 cursor-not-allowed rounded-xl px-3 text-[12px] text-zinc-500", "Заблокировать" }
                        span { id: "dm-block-unavailable", role: "tooltip", class: "pointer-events-none absolute right-0 top-full z-50 mt-1 hidden w-max max-w-60 rounded-lg bg-zinc-800 px-3 py-2 text-[12px] text-zinc-200 shadow-lg group-hover:block group-focus-within:block", "Блокировка в разработке" }
                    }
                }
            }
            if let Some(message) = error() { p { role: "alert", class: "mt-2 text-[12px] text-red-200", "{message}" } }
        }
    }
}

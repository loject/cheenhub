//! Профиль зарегистрированного пользователя и команда назначения владельцем.

use cheenhub_contracts::rest::HostUserSummary;
use dioxus::prelude::*;

use crate::features::app::components::avatar::UserAvatar;

/// Показывает регистрационные данные и доступное первоначальному владельцу действие.
#[component]
pub(super) fn HostUserRow(
    user: HostUserSummary,
    is_owner: bool,
    is_initial_owner: bool,
    can_manage: bool,
    is_granting: bool,
    is_busy: bool,
    on_grant: EventHandler<String>,
) -> Element {
    let date = chrono::DateTime::parse_from_rfc3339(&user.registered_at)
        .map(|date| date.format("%d.%m.%Y, %H:%M UTC").to_string())
        .unwrap_or_else(|_| user.registered_at.clone());

    rsx! {
        div { class: "flex flex-wrap items-center gap-3 py-4",
            UserAvatar {
                nickname: user.nickname.clone(), avatar_url: user.avatar_url.clone(),
                avatar_seed: Some(user.user_id.clone()),
                class: "h-10 w-10 shrink-0 rounded-xl border border-zinc-800 bg-zinc-900 text-[12px] font-bold text-zinc-100".to_owned(),
            }
            div { class: "min-w-0 flex-1",
                div { class: "flex flex-wrap items-center gap-2",
                    p { class: "break-words text-[13px] font-medium text-zinc-100", "{user.nickname}" }
                    if is_owner {
                        span { class: "rounded-full border border-accent/25 bg-accent/10 px-2 py-0.5 text-[10px] font-medium text-blue-200",
                            if is_initial_owner { "Первоначальный владелец" } else { "Владелец хоста" }
                        }
                    }
                }
                p { class: "break-all text-[12px] text-zinc-400", "{user.email}" }
                p { class: "mt-1 text-[11px] tabular-nums text-zinc-500", "Регистрация: {date}" }
            }
            if can_manage && !is_owner {
                button {
                    r#type: "button",
                    class: "inline-flex min-h-10 items-center justify-center rounded-lg border border-accent/30 bg-accent/10 px-3 text-[12px] font-medium text-blue-200 transition-colors hover:bg-accent/20 focus-visible:outline-2 focus-visible:outline-accent disabled:cursor-not-allowed disabled:opacity-50",
                    disabled: is_busy,
                    onclick: move |_| on_grant.call(user.user_id.clone()),
                    if is_granting { "Назначаем..." } else { "Назначить владельцем" }
                }
            }
        }
    }
}

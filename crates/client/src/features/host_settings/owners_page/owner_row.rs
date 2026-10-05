//! Строка владельца хоста в списке.
//!
//! Отзыв требует подтверждения, потому что пользователь сразу теряет доступ
//! ко всем настройкам хоста и должен сначала передать их другому владельцу.

use cheenhub_contracts::rest::HostOwnerSummary;
use dioxus::prelude::*;

use crate::features::app::components::avatar::UserAvatar;

/// Показывает профиль владельца и действие отзыва прав, если оно доступно.
#[component]
pub(super) fn HostOwnerRow(
    owner: HostOwnerSummary,
    can_revoke: bool,
    /// Отзыв прав этой строки ожидает ответ сервера; родитель снимает флаг при любом результате.
    is_revoking: bool,
    on_revoke: EventHandler<String>,
) -> Element {
    let mut confirm = use_signal(|| false);

    let granted_line = match owner.granted_by_nickname.as_deref() {
        Some(granted_by) => format!(
            "Права выдал {granted_by} · {}",
            format_date(&owner.granted_at)
        ),
        None => format!(
            "Первоначальный владелец · {}",
            format_date(&owner.granted_at)
        ),
    };

    rsx! {
        div { class: "flex flex-wrap items-center gap-3 py-3",
            UserAvatar {
                nickname: owner.nickname.clone(),
                avatar_url: owner.avatar_url.clone(),
                class: "h-10 w-10 shrink-0 rounded-xl border border-zinc-800 bg-zinc-900 text-[12px] font-bold text-zinc-100".to_owned(),
                avatar_seed: Some(owner.user_id.clone()),
            }
            div { class: "min-w-0 flex-1",
                div { class: "flex flex-wrap items-center gap-2",
                    p { class: "truncate text-[13px] font-medium text-zinc-100", "{owner.nickname}" }
                    if owner.is_current_user {
                        span { class: "rounded-full border border-accent/25 bg-accent/10 px-2 py-0.5 text-[10px] font-medium text-blue-200",
                            "Ты"
                        }
                    }
                }
                p { class: "truncate text-[12px] text-zinc-500", "{owner.email}" }
                p { class: "text-[11px] text-zinc-600", "{granted_line}" }
            }
            if can_revoke {
                if confirm() {
                    div { class: "flex flex-wrap items-center gap-2",
                        span { class: "text-[12px] text-zinc-400", "Отозвать права?" }
                        button {
                            r#type: "button",
                            class: revoke_button_class(true),
                            disabled: is_revoking,
                            onclick: move |_| {
                                let user_id = owner.user_id.clone();
                                on_revoke.call(user_id);
                            },
                            if is_revoking { "Отзываем..." } else { "Да, отозвать" }
                        }
                        button {
                            r#type: "button",
                            class: cancel_button_class(),
                            disabled: is_revoking,
                            onclick: move |_| confirm.set(false),
                            "Отмена"
                        }
                    }
                } else {
                    button {
                        r#type: "button",
                        class: revoke_button_class(false),
                        onclick: move |_| confirm.set(true),
                        "Отозвать права"
                    }
                }
            }
        }
    }
}

/// Класс кнопки отзыва прав: `active` различает подтверждение и обычное действие.
fn revoke_button_class(active: bool) -> &'static str {
    if active {
        "inline-flex min-h-9 items-center justify-center rounded-lg border border-red-400/30 bg-red-400/15 px-3 text-[12px] font-semibold text-red-200 transition hover:bg-red-400/25 disabled:cursor-not-allowed disabled:opacity-60"
    } else {
        "inline-flex min-h-9 items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900 px-3 text-[12px] font-medium text-zinc-300 transition hover:border-zinc-700 hover:bg-zinc-800"
    }
}

/// Класс кнопки отмены отзыва прав.
fn cancel_button_class() -> &'static str {
    "inline-flex min-h-9 items-center justify-center rounded-lg border border-transparent px-3 text-[12px] font-medium text-zinc-500 transition hover:text-zinc-300"
}

/// Форматирует время выдачи прав в короткий локальный вид.
///
/// Сервер отдаёт RFC 3339 в UTC; детальный разбор не нужен интерфейсу,
/// поэтому дата и время показываются в формате сервера без перевода пояса.
fn format_date(value: &str) -> String {
    value
        .split_once('T')
        .map(|(date, time)| format!("{date}, {}", &time[..time.len().min(5)]))
        .unwrap_or_else(|| value.to_owned())
}

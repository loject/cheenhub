//! Загрузка и постраничный просмотр зарегистрированных пользователей.
//!
//! Страница владеет номером и ресурсом списка; изменение прав остаётся
//! командой родительской страницы, сериализующей выдачу и отзыв.

use cheenhub_contracts::rest::HostOwnerSummary;
use dioxus::prelude::*;

use super::{api, can_manage_owners};

mod user_row;
use user_row::HostUserRow;

/// Показывает одну страницу пользователей с общим количеством и переключением.
#[component]
pub(super) fn HostUsersList(
    owners: Vec<HostOwnerSummary>,
    pending_users: Vec<String>,
    on_grant: EventHandler<String>,
) -> Element {
    let mut page = use_signal(|| 1_u64);
    let mut users = use_resource(move || async move {
        let requested_page = page();
        let result = api::load_users(requested_page).await;
        match &result {
            Ok(response) => debug!(
                page = response.page,
                total = response.total,
                "loaded registered host users"
            ),
            Err(error) => {
                warn!(page = requested_page, message = %error.message(), "failed to load registered host users")
            }
        }
        result
    });
    let loading = *users.state().read() == UseResourceState::Pending;
    let response = users.read().clone();
    let can_manage = can_manage_owners(&owners);

    rsx! {
        div { class: "rounded-[18px] border border-zinc-800 bg-zinc-950/70 p-5",
            h2 { class: "text-[15px] font-semibold text-zinc-100", "Зарегистрированные пользователи" }
            if !can_manage {
                p { class: "mt-1.5 text-pretty text-[13px] leading-5 text-zinc-500",
                    "Назначать владельцев и снимать их права может только первоначальный владелец хоста."
                }
            }
            if loading {
                p { role: "status", class: "mt-5 animate-pulse text-[13px] text-zinc-500", "Загружаем пользователей..." }
            } else {
                match response {
                    Some(Ok(response)) => rsx! {
                        if response.users.is_empty() {
                            div { class: "py-8 text-center text-[13px] text-zinc-400",
                                p { "Пользователи пока не найдены" }
                                button { r#type: "button", class: page_button_class(),
                                    onclick: move |_| users.restart(), "Обновить список"
                                }
                            }
                        } else {
                            div { class: "mt-4 divide-y divide-zinc-800/80",
                                for user in response.users {
                                    HostUserRow {
                                        key: "{user.user_id}",
                                        is_owner: owners.iter().any(|owner| owner.user_id == user.user_id),
                                        is_initial_owner: owners.iter().any(|owner| owner.user_id == user.user_id && owner.is_initial_owner),
                                        is_granting: pending_users.contains(&user.user_id),
                                        is_busy: !pending_users.is_empty(),
                                        can_manage,
                                        user,
                                        on_grant,
                                    }
                                }
                            }
                        }
                        div { class: "mt-5 flex flex-wrap items-center justify-between gap-3 border-t border-zinc-800/80 pt-4",
                            p { class: "text-[12px] tabular-nums text-zinc-500",
                                "Всего: {response.total} · Страница {response.page} из {response.total.div_ceil(response.page_size).max(1)}"
                            }
                            nav { aria_label: "Страницы пользователей", class: "flex gap-2",
                                button { r#type: "button", class: page_button_class(),
                                    disabled: response.page <= 1 || !pending_users.is_empty(),
                                    onclick: move |_| page.set(response.page - 1), "Назад"
                                }
                                button { r#type: "button", class: page_button_class(),
                                    disabled: response.page >= response.total.div_ceil(response.page_size).max(1) || !pending_users.is_empty(),
                                    onclick: move |_| page.set(response.page + 1), "Далее"
                                }
                            }
                        }
                    },
                    Some(Err(error)) => rsx! {
                        p { role: "alert", class: "mt-4 text-[13px] text-red-200", "{error.message()}" }
                        button { r#type: "button", class: page_button_class(),
                            onclick: move |_| users.restart(), "Повторить"
                        }
                    },
                    None => rsx! {},
                }
            }
        }
    }
}

/// Общие состояния кнопок переключения и повтора загрузки списка.
fn page_button_class() -> &'static str {
    "inline-flex min-h-10 items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900 px-3 text-[12px] font-medium text-zinc-300 transition-colors hover:bg-zinc-800 focus-visible:outline-2 focus-visible:outline-accent disabled:cursor-not-allowed disabled:opacity-40"
}

//! Управление владельцами хоста.
//!
//! Страница показывает всех пользователей с глобальными правами и позволяет
//! выдать права новому владельцу или отозвать их у существующего. Отзыв
//! последнего владельца сервер не допускает, поэтому кнопка отзыва у единственного
//! владельца на этом хосте не показывается.

use cheenhub_contracts::rest::{HostOwnerSummary, HostOwnersResponse};
use dioxus::prelude::*;

use super::api;
use super::tabs::{HostSettingsTab, host_settings_tabs};

mod grant_form;
use grant_form::HostOwnerGrantForm;

mod owner_row;
use owner_row::HostOwnerRow;

/// Рендерит список владельцев хоста и форму выдачи прав.
///
/// Список хранится в сигнале, а не только в ресурсе: выдача и отзыв возвращают
/// обновлённый список с сервера, поэтому повторный запрос после каждой операции
/// не нужен.
#[component]
pub(crate) fn HostOwnersPage() -> Element {
    let mut owners_resource = use_resource(api::load_owners);
    let (mut owners, owners_error) = use_owners_load(owners_resource);
    let mut revoke_error = use_signal(|| None::<String>);
    let mut revoke_notice = use_signal(|| None::<String>);
    let mut revoking_owners = use_signal(Vec::<String>::new);

    let revoke_owner = use_callback(move |user_id: String| {
        if revoking_owners.peek().contains(&user_id) {
            return;
        }
        revoking_owners.write().push(user_id.clone());
        revoke_error.set(None);
        revoke_notice.set(None);
        spawn(async move {
            match api::revoke_owner(&user_id).await {
                Ok(remaining) => {
                    info!(%user_id, owner_count = remaining.owners.len(), "revoked host owner rights from host settings");
                    owners.set(remaining.owners);
                    revoke_notice.set(Some(
                        "Права владельца сняты. Пользователь больше не видит настройки хоста."
                            .to_owned(),
                    ));
                }
                Err(failure) => {
                    warn!(%user_id, message = %failure.message(), "failed to revoke host owner rights");
                    revoke_error.set(Some(failure.message().to_owned()));
                    owners_resource.restart();
                }
            }
            revoking_owners
                .write()
                .retain(|pending| pending != &user_id);
        });
    });

    rsx! {
        section { class: "host-settings-scroll min-w-0 flex-1 overflow-y-auto bg-zinc-950/35 px-4 py-6 sm:px-6",
            div { class: "mx-auto w-full max-w-[1180px] pb-10",
                {header_block()}
                {host_settings_tabs(HostSettingsTab::Owners)}
                {owners_body(
                    OwnersView {
                        loading: *owners_resource.state().read() == UseResourceState::Pending,
                        revoking_owners: revoking_owners(),
                        owners: owners(),
                        load_error: owners_error(),
                        revoke_error: revoke_error(),
                        revoke_notice: revoke_notice(),
                    },
                    EventHandler::new(move |user_id: String| revoke_owner.call(user_id)),
                    EventHandler::new(move |response: HostOwnersResponse| {
                        owners.set(response.owners)
                    }),
                    EventHandler::new(move |_| owners_resource.restart()),
                )}
            }
        }
    }
}

/// Заголовок страницы владельцев хоста.
fn header_block() -> Element {
    rsx! {
        div {
            p { class: "text-[11px] font-medium uppercase tracking-[0.20em] text-zinc-600", "Настройки хоста" }
            h1 { class: "mt-1 text-balance text-[22px] font-semibold tracking-[-0.04em] text-zinc-50", "Владельцы хоста" }
            p { class: "mt-1.5 max-w-2xl text-pretty text-[13px] leading-5 text-zinc-500",
                "Владельцы имеют полный доступ к настройкам хоста: журналу сервера, исходящей почте и статистике. Их может быть несколько."
            }
        }
    }
}

/// Состояния страницы, которые нужны разметке, собранные в одну структуру.
///
/// Объединение убирает длинный список параметров у функции разметки и не даёт
/// подписи перепутать местами одинаковые по типу `Option<String>`.
struct OwnersView {
    loading: bool,
    revoking_owners: Vec<String>,
    owners: Vec<HostOwnerSummary>,
    load_error: Option<String>,
    revoke_error: Option<String>,
    revoke_notice: Option<String>,
}

/// Показывает ошибку загрузки или форму выдачи прав вместе со списком владельцев.
///
/// Ошибки выдачи и отзыва показываются под формой, поэтому список остаётся
/// видимым и пользователь может повторить действие без перезагрузки страницы.
fn owners_body(
    view: OwnersView,
    on_revoke: EventHandler<String>,
    on_granted: EventHandler<HostOwnersResponse>,
    on_retry: EventHandler<()>,
) -> Element {
    if view.loading && view.owners.is_empty() {
        return loading_notice();
    }

    if let Some(error) = view.load_error.filter(|_| !view.loading) {
        return rsx! {
            div { class: "mt-6 rounded-[18px] border border-red-400/20 bg-red-400/5 px-5 py-6",
                p { class: "text-pretty text-[13px] text-red-200", "{error}" }
                button {
                    r#type: "button",
                    class: "mt-4 inline-flex min-h-10 items-center rounded-xl border border-zinc-800 bg-zinc-900 px-4 text-[13px] font-medium text-zinc-200 transition hover:bg-zinc-800",
                    onclick: move |_| on_retry.call(()),
                    "Повторить"
                }
            }
        };
    }

    rsx! {
        div { class: "mt-6 space-y-6",
            if view.loading { {loading_notice()} }
            HostOwnerGrantForm { on_granted }
            {status_notice(view.revoke_notice, view.revoke_error)}
            {owner_list(&view.owners, &view.revoking_owners, on_revoke)}
        }
    }
}

/// Показывает ожидание первоначальной загрузки или обновления списка владельцев.
fn loading_notice() -> Element {
    rsx! {
        div { class: "animate-pulse rounded-[18px] border border-zinc-800 bg-zinc-950/70 px-5 py-6",
            p { role: "status", class: "text-[13px] text-zinc-500", "Загружаем владельцев..." }
        }
    }
}

/// Показывает результат последней операции отзыва прав.
fn status_notice(notice: Option<String>, error: Option<String>) -> Element {
    if let Some(error) = error {
        return rsx! {
            p { class: "rounded-xl bg-red-400/10 px-3 py-2 text-pretty text-[12px] leading-5 text-red-200", "{error}" }
        };
    }
    let Some(notice) = notice else {
        return rsx! {};
    };

    rsx! {
        p { class: "rounded-xl bg-emerald-400/10 px-3 py-2 text-pretty text-[12px] leading-5 text-emerald-200", "{notice}" }
    }
}

/// Список владельцев: карточка владельца на каждого или пустое состояние.
fn owner_list(
    owners: &[HostOwnerSummary],
    revoking_owners: &[String],
    on_revoke: EventHandler<String>,
) -> Element {
    if owners.is_empty() {
        return rsx! {
            div { class: "rounded-[18px] border border-zinc-800 bg-zinc-950/70 px-5 py-8 text-center",
                p { class: "text-[13px] font-medium text-zinc-300", "Пока нет ни одного владельца" }
                p { class: "mx-auto mt-1.5 max-w-md text-pretty text-[13px] leading-5 text-zinc-500",
                    "Укажи email или идентификатор пользователя выше, чтобы выдать права владельца хоста."
                }
            }
        };
    }

    rsx! {
        div { class: "rounded-[18px] border border-zinc-800 bg-zinc-950/70 p-5",
            h2 { class: "text-[15px] font-semibold text-zinc-100", "Владельцы" }
            p { class: "mt-1.5 text-pretty text-[13px] leading-5 text-zinc-500",
                "Каждый из этих пользователей может менять любые настройки этого хоста."
            }
            div { class: "mt-4 divide-y divide-zinc-800/80",
                for owner in owners {
                    HostOwnerRow {
                        key: "{owner.user_id}",
                        owner: owner.clone(),
                        can_revoke: !owner.is_current_user && owners.len() > 1,
                        is_revoking: revoking_owners.contains(&owner.user_id),
                        on_revoke,
                    }
                }
            }
        }
    }
}

/// Применяет ответы загрузки к локальному списку, изменяемому операциями выдачи и отзыва.
fn use_owners_load(
    owners_resource: Resource<Result<HostOwnersResponse, api::HostSettingsApiError>>,
) -> (Signal<Vec<HostOwnerSummary>>, Signal<Option<String>>) {
    let mut owners = use_signal(Vec::<HostOwnerSummary>::new);
    let mut owners_error = use_signal(|| None::<String>);

    use_effect(move || {
        let Some(loaded) = owners_resource.read().clone() else {
            return;
        };
        match loaded {
            Ok(response) => {
                debug!(
                    owner_count = response.owners.len(),
                    "applying loaded host owners"
                );
                owners.set(response.owners);
                owners_error.set(None);
            }
            Err(failure) => {
                warn!(message = %failure.message(), "failed to load host owners in settings");
                owners_error.set(Some(failure.message().to_owned()));
            }
        }
    });

    (owners, owners_error)
}

#[cfg(test)]
mod tests;

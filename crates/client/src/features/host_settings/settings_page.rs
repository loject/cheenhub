//! Системные настройки хоста: уровень журналирования сервера.
//!
//! Страница меняет минимальный уровень журнала, который сервер применяет ко
//! всем своим событиям, поэтому выбор действует сразу и переживает перезапуск.

use cheenhub_contracts::rest::HostLogSettingsResponse;
use dioxus::prelude::*;

use super::api;

mod registration_form;
mod registration_panel;
mod settings_form;
use settings_form::HostLogSettingsForm;

/// Рендерит настройки уровня журналирования, доступные владельцу хоста.
#[component]
pub(crate) fn HostSystemSettingsPage() -> Element {
    let mut settings_resource = use_resource(api::load_log_settings);
    let settings_result = settings_resource.read().clone();
    settings_content(
        settings_result,
        EventHandler::new(move |_| settings_resource.restart()),
    )
}

/// Рендерит загрузку, ошибку или доступную форму настроек журнала.
fn settings_content(
    settings_result: Option<Result<HostLogSettingsResponse, api::HostSettingsApiError>>,
    onretry: EventHandler<()>,
) -> Element {
    match settings_result {
        None => rsx! {
            section { class: page_class(),
                div { class: content_class(),
                    div { class: "animate-pulse rounded-[18px] border border-zinc-800 bg-zinc-950/70 px-5 py-6",
                        p { class: "text-[13px] text-zinc-600", "Загружаем настройки журнала..." }
                    }
                }
            }
        },
        Some(Err(failure)) => rsx! {
            section { class: page_class(),
                div { class: content_class(),
                    div { class: "rounded-[18px] border border-red-400/20 bg-red-400/5 px-5 py-6",
                        p { class: "text-pretty text-[13px] text-red-200", "{failure.message()}" }
                        button {
                            r#type: "button",
                            class: "mt-4 inline-flex min-h-10 items-center rounded-xl border border-zinc-800 bg-zinc-900 px-4 text-[13px] font-medium text-zinc-200 transition hover:bg-zinc-800",
                            onclick: move |_| onretry.call(()),
                            "Повторить"
                        }
                    }
                }
            }
        },
        Some(Ok(settings)) => rsx! { HostLogSettingsForm { settings } },
    }
}

/// Классы прокручиваемой обёртки страницы настроек хоста.
fn page_class() -> &'static str {
    "host-settings-scroll min-w-0 flex-1 overflow-y-auto bg-zinc-950/35 px-4 py-6 sm:px-6"
}

/// Классы ограниченной по ширине колонки содержимого страницы.
fn content_class() -> &'static str {
    "mx-auto w-full max-w-[1180px] pb-10"
}

/// Заголовок страницы системных настроек.
fn header_block() -> Element {
    rsx! {
        div {
            p { class: "text-[11px] font-medium uppercase tracking-[0.20em] text-zinc-600", "Настройки хоста" }
            h1 { class: "mt-1 text-balance text-[22px] font-semibold tracking-[-0.04em] text-zinc-50", "Системные настройки" }
            p { class: "mt-1.5 max-w-2xl text-pretty text-[13px] leading-5 text-zinc-500",
                "Управляй журналом сервера и доступностью регистрации пользователей."
            }
        }
    }
}

#[cfg(test)]
mod tests;

//! Загрузка настроек способов регистрации.

use dioxus::prelude::*;

use super::super::api;
use super::registration_form::HostRegistrationSettingsForm;
/// Загружает и показывает настройки доступности регистрации.
#[component]
pub(super) fn HostRegistrationSettingsPanel() -> Element {
    let mut settings_resource = use_resource(api::load_registration_settings);
    let settings_result = settings_resource.read().clone();

    match settings_result {
        None => rsx! {
            div { class: "mt-4 rounded-[18px] border border-zinc-800 bg-zinc-950/70 p-5",
                p { class: "animate-pulse text-[13px] text-zinc-600", "Загружаем настройки регистрации..." }
            }
        },
        Some(Err(failure)) => rsx! {
            div { class: "mt-4 rounded-[18px] border border-red-400/20 bg-red-400/5 p-5",
                p { class: "text-pretty text-[13px] text-red-200", "Не удалось загрузить настройки регистрации: {failure.message()}" }
                button {
                    r#type: "button",
                    class: "mt-4 inline-flex min-h-10 items-center rounded-xl border border-zinc-800 bg-zinc-900 px-4 text-[13px] font-medium text-zinc-200 transition hover:bg-zinc-800",
                    onclick: move |_| settings_resource.restart(),
                    "Повторить"
                }
            }
        },
        Some(Ok(settings)) => rsx! { HostRegistrationSettingsForm { settings } },
    }
}

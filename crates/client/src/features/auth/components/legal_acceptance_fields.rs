//! Отдельные подтверждения юридических документов в форме регистрации.

use dioxus::prelude::*;

use crate::Route;

/// Изменение одного из юридически значимых подтверждений регистрации.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum LegalAcceptanceAction {
    /// Пользователь изменил принятие пользовательского соглашения.
    TermsChanged(bool),
    /// Пользователь изменил согласие на обработку персональных данных.
    PersonalDataChanged(bool),
}

/// Показывает отдельные флажки соглашения и согласия на обработку данных.
#[component]
pub(crate) fn LegalAcceptanceFields(
    accepts_terms: bool,
    accepts_personal_data: bool,
    on_change: EventHandler<LegalAcceptanceAction>,
) -> Element {
    rsx! {
        fieldset { class: "space-y-3",
            legend { class: "sr-only", "Обязательные документы" }
            div { class: "flex items-start gap-3 text-[12px] leading-5 text-zinc-500",
                input {
                    id: "accepts-terms",
                    r#type: "checkbox",
                    required: true,
                    checked: accepts_terms,
                    class: "mt-0.5 h-4 w-4 shrink-0 cursor-pointer rounded border-zinc-700 bg-zinc-900 accent-blue-500",
                    onchange: move |event| on_change.call(LegalAcceptanceAction::TermsChanged(event.checked())),
                }
                label { r#for: "accepts-terms", class: "cursor-pointer",
                    "Я принимаю "
                    Link { to: Route::Terms { return_to: Some("registration".to_string()) }, class: "text-zinc-200 underline decoration-zinc-700 underline-offset-2 transition hover:text-white", "Пользовательское соглашение" }
                    "."
                }
            }
            div { class: "flex items-start gap-3 text-[12px] leading-5 text-zinc-500",
                input {
                    id: "accepts-personal-data",
                    r#type: "checkbox",
                    required: true,
                    checked: accepts_personal_data,
                    class: "mt-0.5 h-4 w-4 shrink-0 cursor-pointer rounded border-zinc-700 bg-zinc-900 accent-blue-500",
                    onchange: move |event| on_change.call(LegalAcceptanceAction::PersonalDataChanged(event.checked())),
                }
                label { r#for: "accepts-personal-data", class: "cursor-pointer",
                    "Я даю отдельное "
                    Link { to: Route::PersonalDataConsent { return_to: Some("registration".to_string()) }, class: "text-zinc-200 underline decoration-zinc-700 underline-offset-2 transition hover:text-white", "Согласие на обработку персональных данных" }
                    " и подтверждаю, что ознакомился с "
                    Link { to: Route::PrivacyPolicy { return_to: Some("registration".to_string()) }, class: "text-zinc-200 underline decoration-zinc-700 underline-offset-2 transition hover:text-white", "Политикой обработки персональных данных" }
                    "."
                }
            }
            p { class: "ml-4 text-[11px] leading-4 text-zinc-600",
                "Регистрация доступна пользователям старше 18 лет."
            }
        }
    }
}

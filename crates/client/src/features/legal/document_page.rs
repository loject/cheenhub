//! Общая страница для публичных юридических документов.

use cheenhub_contracts::legal::LEGAL_EFFECTIVE_DATE;
use dioxus::prelude::*;

use crate::Route;

/// Содержательный раздел юридического документа.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct LegalSection {
    /// Заголовок раздела.
    pub(crate) title: &'static str,
    /// Абзацы перед перечнем.
    pub(crate) paragraphs: &'static [&'static str],
    /// Элементы перечня, если они нужны разделу.
    pub(crate) bullets: &'static [&'static str],
    /// Выделенное пояснение после основного текста.
    pub(crate) note: Option<&'static str>,
}

/// Данные отдельного публичного юридического документа.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct LegalDocument {
    /// Стабильный идентификатор документа для диагностического события.
    pub(crate) key: &'static str,
    /// Название документа.
    pub(crate) title: &'static str,
    /// Версия документа из общего контракта.
    pub(crate) version: &'static str,
    /// Краткое вводное пояснение.
    pub(crate) lead: &'static str,
    /// Разделы документа.
    pub(crate) sections: &'static [LegalSection],
}

/// Показывает юридический документ с общей навигацией и реквизитами оператора.
#[component]
pub(crate) fn LegalDocumentPage(document: LegalDocument, return_to_registration: bool) -> Element {
    use_effect(move || {
        info!(
            document = document.key,
            version = document.version,
            return_to_registration,
            "opened public legal document"
        );
    });

    rsx! {
        div { class: "min-h-screen bg-zinc-950 text-zinc-100 selection:bg-zinc-700/40",
            header { class: "sticky top-0 z-30 border-b border-zinc-800/80 bg-zinc-950/90 backdrop-blur-xl",
                div { class: "mx-auto flex max-w-5xl items-center justify-between gap-4 px-5 py-3 lg:px-8",
                    Link {
                        to: Route::Landing {},
                        class: "flex min-w-0 items-center gap-3",
                        span { class: "grid h-8 w-8 shrink-0 place-items-center rounded-xl border border-zinc-700 bg-zinc-100 text-[11px] font-bold text-zinc-950", "CH" }
                        span { class: "truncate text-sm font-semibold text-zinc-50", "CheenHub" }
                    }
                    if return_to_registration {
                        Link {
                            to: Route::Register {},
                            class: "rounded-lg border border-blue-500/40 bg-blue-500/10 px-3 py-1.5 text-[12px] font-medium text-blue-100 transition hover:border-blue-400/70 hover:bg-blue-500/20",
                            "Вернуться к регистрации"
                        }
                    } else {
                        Link {
                            to: Route::Landing {},
                            class: "rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-[12px] font-medium text-zinc-300 transition hover:border-zinc-700 hover:text-zinc-100",
                            "На главную"
                        }
                    }
                }
            }

            main { class: "relative overflow-hidden px-5 py-10 sm:py-14 lg:px-8",
                div { class: "pointer-events-none absolute left-1/2 top-0 h-72 w-full max-w-5xl -translate-x-1/2 rounded-full bg-accent/5 blur-3xl" }
                article { class: "relative mx-auto max-w-3xl",
                    div { class: "mb-8 border-b border-zinc-800 pb-8",
                        p { class: "text-[11px] font-semibold uppercase tracking-[0.22em] text-blue-300", "Документы CheenHub" }
                        h1 { class: "mt-3 text-3xl font-semibold tracking-[-0.035em] text-zinc-50 sm:text-4xl", "{document.title}" }
                        p { class: "mt-4 max-w-2xl text-[15px] leading-7 text-zinc-400", "{document.lead}" }
                        div { class: "mt-5 flex flex-wrap gap-2 text-[12px] text-zinc-400",
                            span { class: "rounded-full border border-zinc-800 bg-zinc-900/80 px-3 py-1.5", "Версия: {document.version}" }
                            span { class: "rounded-full border border-zinc-800 bg-zinc-900/80 px-3 py-1.5", "Действует с {LEGAL_EFFECTIVE_DATE}" }
                        }
                    }

                    div { class: "space-y-9",
                        for section in document.sections {
                            section { class: "scroll-mt-24",
                                h2 { class: "text-xl font-semibold tracking-[-0.02em] text-zinc-100",
                                    "{section.title}"
                                }
                                div { class: "mt-3 space-y-3 text-[15px] leading-7 text-zinc-400",
                                    for paragraph in section.paragraphs {
                                        p { "{paragraph}" }
                                    }
                                    if !section.bullets.is_empty() {
                                        ul { class: "ml-4 space-y-2",
                                            for item in section.bullets {
                                                li { class: "list-disc marker:text-zinc-600", "{item}" }
                                            }
                                        }
                                    }
                                    if let Some(note) = section.note {
                                        p { class: "rounded-xl border border-zinc-800 bg-zinc-900/70 px-4 py-3 text-zinc-300", "{note}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            footer { class: "border-t border-zinc-800/80 px-5 py-8 lg:px-8",
                div { class: "mx-auto flex max-w-3xl flex-col gap-5 text-[12px] text-zinc-500",
                    nav { class: "flex flex-wrap gap-x-5 gap-y-2",
                        Link { to: document_route(Route::Terms { return_to: None }, return_to_registration), class: "transition hover:text-zinc-200", "Пользовательское соглашение" }
                        Link { to: document_route(Route::PrivacyPolicy { return_to: None }, return_to_registration), class: "transition hover:text-zinc-200", "Политика обработки данных" }
                        Link { to: document_route(Route::PersonalDataConsent { return_to: None }, return_to_registration), class: "transition hover:text-zinc-200", "Согласие на обработку данных" }
                    }
                    p {
                        "Вопросы по документам и персональным данным: "
                        a { href: "mailto:cheengeez13@gmail.com", class: "text-zinc-300 transition hover:text-zinc-100", "cheengeez13@gmail.com" }
                    }
                }
            }
        }
    }
}

/// Формирует маршрут документа, сохраняя переход из регистрации.
fn document_route(route: Route, return_to_registration: bool) -> Route {
    if !return_to_registration {
        return route;
    }

    match route {
        Route::Terms { .. } => Route::Terms {
            return_to: Some("registration".to_string()),
        },
        Route::PrivacyPolicy { .. } => Route::PrivacyPolicy {
            return_to: Some("registration".to_string()),
        },
        Route::PersonalDataConsent { .. } => Route::PersonalDataConsent {
            return_to: Some("registration".to_string()),
        },
        _ => route,
    }
}

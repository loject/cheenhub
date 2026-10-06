//! Форма журналирования, владеющая выбором и последним сохранённым ответом.
//!
//! Форма монтируется только после загрузки настроек; успешный PATCH обновляет
//! локальную базу сравнения вместе с уровнем и временем изменения.

use cheenhub_contracts::rest::{
    HostLogLevel, HostLogSettingsResponse, UpdateHostLogSettingsRequest,
};
use dioxus::prelude::*;

use super::super::api;
use super::super::tabs::{HostSettingsTab, host_settings_tabs};
use super::{content_class, header_block, page_class};

/// Уровни журнала от наименее подробного к самому подробному.
const LEVELS: [(HostLogLevel, &str, &str); 5] = [
    (HostLogLevel::Error, "Ошибки", "Только непредвиденные сбои"),
    (
        HostLogLevel::Warn,
        "Предупреждения",
        "Сбои и подозрительные ситуации",
    ),
    (
        HostLogLevel::Info,
        "Обычные события",
        "Основные действия пользователей и системы",
    ),
    (
        HostLogLevel::Debug,
        "Отладка",
        "Технические детали запросов и соединений",
    ),
    (
        HostLogLevel::Trace,
        "Трассировка",
        "Все внутренние вызовы и нагрузка на процесс",
    ),
];

/// Значение формы для фильтра, заданного при запуске сервера.
const STARTUP_FILTER_OPTION: &str = "startup";

/// Состояние сохранения выбранного уровня журнала.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SaveState {
    Idle,
    Saving,
    Saved,
    Failed,
}

/// Показывает загруженные настройки и управляет сохранением уровня журнала.
#[component]
pub(super) fn HostLogSettingsForm(settings: HostLogSettingsResponse) -> Element {
    let initial_selection = current_option(&settings);
    let selected = use_signal(|| initial_selection);
    let saved_settings = use_signal(|| settings);
    let save_state = use_signal(|| SaveState::Idle);
    let error = use_signal(|| None::<String>);
    let saving = save_state() == SaveState::Saving;
    let changed = selected() != current_option(&saved_settings());

    rsx! {
        section { class: page_class(),
            div { class: content_class(),
                {header_block()}
                {host_settings_tabs(HostSettingsTab::Settings)}
                {level_card(&saved_settings(), saved_settings, selected, save_state, error, changed, saving)}
            }
        }
    }
}

/// Карточка выбора минимального уровня журнала.
///
/// Компонент принимает состояние выбора и сохранения и отдаёт наружу только
/// разметку: изменения уровня и запрос сохранения принадлежат форме.
fn level_card(
    settings: &HostLogSettingsResponse,
    saved_settings: Signal<HostLogSettingsResponse>,
    mut selected: Signal<String>,
    mut save_state: Signal<SaveState>,
    mut error: Signal<Option<String>>,
    changed: bool,
    saving: bool,
) -> Element {
    rsx! {
        div { class: "mt-6 rounded-[18px] border border-zinc-800 bg-zinc-950/70 p-5",
            h2 { class: "text-[15px] font-semibold text-zinc-100", "Уровень журнала" }
            p { class: "mt-1.5 text-pretty text-[13px] leading-5 text-zinc-500",
                "В журнал попадают события выбранного уровня и более серьёзные. Уровень применяется сразу после сохранения и сохраняется для следующих запусков сервера."
            }

            div { class: "mt-5 grid gap-2 sm:grid-cols-2 xl:grid-cols-3",
                for level in LEVELS {
                    {level_option_button(level, selected)}
                }
            }

            button {
                r#type: "button",
                class: startup_option_class(selected() == STARTUP_FILTER_OPTION),
                disabled: selected() == STARTUP_FILTER_OPTION,
                onclick: move |_| selected.set(STARTUP_FILTER_OPTION.to_owned()),
                div { class: "min-w-0",
                    p { class: "text-[13px] font-medium text-zinc-100", "Как настроено при запуске" }
                    p { class: "mt-0.5 text-[12px] text-zinc-500",
                        "Вернуть фильтр, заданный конфигурацией сервера"
                    }
                }
            }

            div { class: "mt-5 flex flex-col gap-3 border-t border-zinc-800 pt-4 sm:flex-row sm:items-center sm:justify-between",
                div { class: "min-h-5 text-pretty text-[12px]",
                    match save_state() {
                        SaveState::Idle => rsx! {
                            span { class: "text-zinc-500", "Уровень применяется ко всем событиям сервера." }
                        },
                        SaveState::Saving => rsx! { span { class: "text-blue-200", "Сохраняем..." } },
                        SaveState::Saved => rsx! {
                            span { class: "text-emerald-200", "Уровень журнала обновлён." }
                        },
                        SaveState::Failed => rsx! {
                            span { class: "text-red-200", "{error().unwrap_or_default()}" }
                        },
                    }
                }
                button {
                    r#type: "button",
                    disabled: !changed || saving,
                    class: apply_button_class(changed && !saving),
                    onclick: move |_| {
                        let requested = selected();
                        save_state.set(SaveState::Saving);
                        error.set(None);
                        spawn(async move {
                            let request = UpdateHostLogSettingsRequest {
                                min_level: parse_level(&requested),
                            };
                            match api::update_log_settings(request).await {
                                Ok(saved) => {
                                    info!(min_level = ?saved.min_level, "host log level updated");
                                    complete_save(saved, selected, save_state, saved_settings);
                                }
                                Err(failure) => {
                                    warn!(message = %failure.message(), "failed to update host log level");
                                    error.set(Some(failure.message().to_owned()));
                                    save_state.set(SaveState::Failed);
                                }
                            }
                        });
                    },
                    if saving { "Сохраняем..." } else { "Применить уровень" }
                }
            }

            if settings.updated_at.is_some() {
                p { class: "mt-3 text-[12px] text-zinc-600",
                    "Последнее изменение уже действует и сохранится после перезапуска сервера."
                }
            }
        }
    }
}

/// Применяет успешный ответ сохранения к локальному состоянию формы.
fn complete_save(
    saved: HostLogSettingsResponse,
    mut selected: Signal<String>,
    mut save_state: Signal<SaveState>,
    mut saved_settings: Signal<HostLogSettingsResponse>,
) {
    selected.set(current_option(&saved));
    saved_settings.set(saved);
    save_state.set(SaveState::Saved);
}

/// Рендежит одну карточку выбора уровня журнала.
fn level_option_button(
    (level, title, hint): (HostLogLevel, &'static str, &'static str),
    mut selected: Signal<String>,
) -> Element {
    let option = level_option(level);
    let active = selected() == option;

    rsx! {
        button {
            r#type: "button",
            class: level_option_class(active),
            onclick: move |_| selected.set(option.clone()),
            div { class: "flex items-center justify-between gap-2",
                p { class: "text-[13px] font-medium text-zinc-100", "{title}" }
                span { class: "text-[10px] font-semibold uppercase tracking-[0.12em] text-zinc-500", "{option}" }
            }
            p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "{hint}" }
        }
    }
}

/// Класс кнопки сохранения уровня журнала.
fn apply_button_class(enabled: bool) -> &'static str {
    if enabled {
        "inline-flex min-h-11 items-center justify-center rounded-xl bg-accent px-5 text-[13px] font-semibold text-white shadow-[0_8px_28px_rgba(59,130,246,0.18)] transition-[background-color,transform] duration-150 hover:bg-blue-400 active:scale-[0.97]"
    } else {
        "inline-flex min-h-11 cursor-not-allowed items-center justify-center rounded-xl bg-zinc-800 px-5 text-[13px] font-semibold text-zinc-500"
    }
}

/// Класс карточки уровня журнала в зависимости от выбора.
fn level_option_class(active: bool) -> &'static str {
    if active {
        "rounded-xl border border-accent/25 bg-accent/10 px-4 py-3 text-left transition-[background-color,border-color,transform] duration-150 active:scale-[0.98]"
    } else {
        "rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-left transition-[background-color,border-color,transform] duration-150 hover:border-zinc-700 hover:bg-zinc-900 active:scale-[0.98]"
    }
}

/// Класс строки возврата к фильтру запуска сервера.
fn startup_option_class(active: bool) -> &'static str {
    if active {
        "mt-2 w-full cursor-not-allowed rounded-xl border border-accent/25 bg-accent/10 px-4 py-3 text-left opacity-70"
    } else {
        "mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-left transition-[background-color,border-color,transform] duration-150 hover:border-zinc-700 hover:bg-zinc-900 active:scale-[0.98]"
    }
}

/// Возвращает строку выбора; `startup` означает фильтр запуска.
fn current_option(settings: &HostLogSettingsResponse) -> String {
    settings
        .min_level
        .map(level_option)
        .unwrap_or_else(|| STARTUP_FILTER_OPTION.to_owned())
}

/// Строка формы, соответствующая уровню журнала.
fn level_option(level: HostLogLevel) -> String {
    match level {
        HostLogLevel::Error => "error",
        HostLogLevel::Warn => "warn",
        HostLogLevel::Info => "info",
        HostLogLevel::Debug => "debug",
        HostLogLevel::Trace => "trace",
    }
    .to_owned()
}

/// Разбирает выбранную строку формы обратно в уровень журнала.
fn parse_level(value: &str) -> Option<HostLogLevel> {
    match value {
        "error" => Some(HostLogLevel::Error),
        "warn" => Some(HostLogLevel::Warn),
        "info" => Some(HostLogLevel::Info),
        "debug" => Some(HostLogLevel::Debug),
        "trace" => Some(HostLogLevel::Trace),
        _ => None,
    }
}

#[cfg(test)]
mod tests;

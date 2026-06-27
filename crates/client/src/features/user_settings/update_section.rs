//! Блок проверки обновлений в системных настройках пользователя.

use dioxus::prelude::*;

use crate::features::application_update::{
    ApplicationUpdateHandle, AvailableUpdate, UpdateUiStatus,
};
use crate::features::toast::ToastHandle;

/// Рендерит настройки проверки обновлений CheenHub.
#[component]
pub(crate) fn UpdateSettingsSection() -> Element {
    let update = use_context::<ApplicationUpdateHandle>();
    let toast = use_context::<ToastHandle>();
    let status = update.ui_status();
    let is_checking = matches!(status, UpdateUiStatus::Checking);
    let panel_class = status_panel_class(&status);

    rsx! {
        div { class: "rounded-2xl border border-zinc-800 bg-zinc-900/45 p-4",
            div { class: "flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between",
                div {
                    h4 { class: "text-[14px] font-semibold text-zinc-100", "Обновления" }
                    p { class: "mt-1 text-[12px] leading-5 text-zinc-500",
                        "Текущая версия CheenHub: {update.current_version()}"
                    }
                }
                button {
                    r#type: "button",
                    disabled: is_checking,
                    class: check_button_class(is_checking),
                    onclick: move |_| {
                        update.check_now();
                        toast.info("Проверяем GitHub Releases.");
                    },
                    if is_checking { "Проверяем..." } else { "Проверить обновления" }
                }
            }

            div { class: panel_class,
                match status {
                    UpdateUiStatus::Idle => rsx! {
                        p { class: "text-[13px] font-medium text-zinc-200", "Проверка еще не выполнялась" }
                        p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Нажмите кнопку, чтобы проверить последний релиз на GitHub." }
                    },
                    UpdateUiStatus::Checking => rsx! {
                        div { class: "flex items-center gap-2",
                            span { class: "inline-block h-3.5 w-3.5 animate-spin rounded-full border-2 border-blue-300/30 border-t-blue-200" }
                            p { class: "text-[13px] font-medium text-blue-100", "Проверяем GitHub Releases..." }
                        }
                    },
                    UpdateUiStatus::Current { checked_at_epoch_seconds } => rsx! {
                        p { class: "text-[13px] font-medium text-emerald-100", "Установлена актуальная версия" }
                        p { class: "mt-1 text-[12px] leading-5 text-emerald-200/75",
                            "Последняя проверка: {format_epoch_time(checked_at_epoch_seconds)}."
                        }
                    },
                    UpdateUiStatus::Available { update: available_update } => rsx! {
                        {available_update_panel(available_update, toast)}
                    },
                    UpdateUiStatus::Deferred { update: available_update, until_epoch_seconds } => rsx! {
                        {deferred_update_panel(available_update, until_epoch_seconds, update, toast)}
                    },
                    UpdateUiStatus::Failed { ref message } => rsx! {
                        p { class: "text-[13px] font-medium text-red-100", "Не удалось проверить обновления" }
                        p { class: "mt-1 text-[12px] leading-5 text-red-200/80", "{message}" }
                    },
                }
            }
        }
    }
}

fn available_update_panel(update: AvailableUpdate, toast: ToastHandle) -> Element {
    let version = update.version.clone();
    let install_version = update.version.clone();
    let install_release_url = update.release_url.clone();

    rsx! {
        div {
            p { class: "text-[13px] font-medium text-blue-100",
                "Доступна новая версия {version}"
            }
            if let Some(title) = update.title.as_ref() {
                p { class: "mt-1 text-[12px] leading-5 text-blue-100/75", "{title}" }
            } else {
                p { class: "mt-1 text-[12px] leading-5 text-blue-100/75", "На GitHub опубликован новый релиз CheenHub." }
            }
            div { class: "mt-3 flex flex-col gap-2 sm:flex-row sm:items-center",
                a {
                    href: "{update.release_url}",
                    target: "_blank",
                    rel: "noreferrer",
                    class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-semibold text-zinc-200 transition hover:border-zinc-700 hover:bg-zinc-900",
                    "Открыть релиз"
                }
                button {
                    r#type: "button",
                    class: "flex h-9 items-center justify-center rounded-xl border border-blue-400/25 bg-blue-500/10 px-3 text-[12px] font-semibold text-blue-100 transition hover:border-blue-400/40 hover:bg-blue-500/15",
                    onclick: move |_| {
                        info!(
                            update_version = %install_version,
                            release_url = %install_release_url,
                            "application update install requested from settings while installer is not implemented"
                        );
                        toast.info("Скачивание и установка будут подключены отдельным шагом.");
                    },
                    "Скачать и установить"
                }
            }
        }
    }
}

fn deferred_update_panel(
    update: AvailableUpdate,
    until_epoch_seconds: u64,
    handle: ApplicationUpdateHandle,
    toast: ToastHandle,
) -> Element {
    let version = update.version.clone();
    let install_version = update.version.clone();
    let install_release_url = update.release_url.clone();

    rsx! {
        div {
            p { class: "text-[13px] font-medium text-amber-100",
                "Вы попросили отложить обновление до версии {version}"
            }
            p { class: "mt-1 text-[12px] leading-5 text-amber-100/80",
                "Покажем уведомление {format_remaining(until_epoch_seconds)}."
            }
            div { class: "mt-3 flex flex-col gap-2 sm:flex-row",
                button {
                    r#type: "button",
                    class: "flex h-9 items-center justify-center rounded-xl border border-amber-300/25 bg-amber-400/10 px-3 text-[12px] font-semibold text-amber-100 transition hover:border-amber-300/40 hover:bg-amber-400/15",
                    onclick: move |_| handle.show_deferred_update_now(),
                    "Показать сейчас"
                }
                button {
                    r#type: "button",
                    class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-semibold text-zinc-200 transition hover:border-zinc-700 hover:bg-zinc-900",
                    onclick: move |_| {
                        info!(
                            update_version = %install_version,
                            release_url = %install_release_url,
                            "application update install requested from settings while installer is not implemented"
                        );
                        toast.info("Скачивание и установка будут подключены отдельным шагом.");
                    },
                    "Скачать и установить"
                }
            }
        }
    }
}

fn check_button_class(is_checking: bool) -> &'static str {
    if is_checking {
        "flex h-10 w-full shrink-0 cursor-wait items-center justify-center rounded-xl border border-blue-400/25 bg-blue-500/10 px-3 text-[12px] font-semibold text-blue-100 disabled:opacity-70 sm:h-9 sm:w-auto"
    } else {
        "flex h-10 w-full shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-semibold text-zinc-200 transition hover:border-zinc-700 hover:bg-zinc-900 sm:h-9 sm:w-auto"
    }
}

fn status_panel_class(status: &UpdateUiStatus) -> &'static str {
    match status {
        UpdateUiStatus::Current { .. } => {
            "mt-4 rounded-2xl border border-emerald-400/20 bg-emerald-500/10 p-4"
        }
        UpdateUiStatus::Available { .. } => {
            "mt-4 rounded-2xl border border-blue-400/20 bg-blue-500/10 p-4"
        }
        UpdateUiStatus::Deferred { .. } => {
            "mt-4 rounded-2xl border border-amber-300/20 bg-amber-400/10 p-4"
        }
        UpdateUiStatus::Failed { .. } => {
            "mt-4 rounded-2xl border border-red-500/20 bg-red-500/10 p-4"
        }
        UpdateUiStatus::Checking | UpdateUiStatus::Idle => {
            "mt-4 rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4"
        }
    }
}

fn format_epoch_time(epoch_seconds: u64) -> String {
    let minutes = (epoch_seconds / 60) % 60;
    let hours = (epoch_seconds / 3_600) % 24;
    format!("{hours:02}:{minutes:02} UTC")
}

fn format_remaining(until_epoch_seconds: u64) -> String {
    let now = crate::features::application_update::now_epoch_seconds();
    let remaining = until_epoch_seconds.saturating_sub(now);
    if remaining == 0 {
        return "сейчас".to_owned();
    }

    let hours = remaining / 3_600;
    let minutes = (remaining % 3_600) / 60;
    if hours >= 24 {
        let days = hours / 24;
        return format!("через {days} дн.");
    }
    if hours > 0 {
        return format!("через {hours} ч {minutes} мин");
    }
    format!("через {minutes} мин")
}

//! Блок проверки обновлений в системных настройках пользователя.

use dioxus::prelude::*;

use crate::features::application_update::{
    ApplicationUpdateHandle, ApplicationUpdateShutdown, AvailableUpdate, UpdateDownloadProgress,
    UpdateDownloadStatus, UpdateUiStatus, use_application_update_shutdown,
};
use crate::features::toast::ToastHandle;

/// Рендерит настройки проверки обновлений CheenHub.
#[component]
pub(crate) fn UpdateSettingsSection() -> Element {
    let update = use_context::<ApplicationUpdateHandle>();
    let toast = use_context::<ToastHandle>();
    let update_shutdown = use_application_update_shutdown();
    let status = update.ui_status();
    let download_status = update.download_status();
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
                        {available_update_panel(available_update, update, download_status, toast, update_shutdown.clone())}
                    },
                    UpdateUiStatus::Deferred { update: available_update, until_epoch_seconds } => rsx! {
                        {deferred_update_panel(available_update, until_epoch_seconds, update, download_status, toast, update_shutdown.clone())}
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

fn available_update_panel(
    update: AvailableUpdate,
    handle: ApplicationUpdateHandle,
    download_status: UpdateDownloadStatus,
    toast: ToastHandle,
    update_shutdown: ApplicationUpdateShutdown,
) -> Element {
    let version = update.version.clone();

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
                {download_update_button(&update, handle, &download_status, toast, update_shutdown)}
            }
            {download_status_panel(&download_status)}
        }
    }
}

fn deferred_update_panel(
    update: AvailableUpdate,
    until_epoch_seconds: u64,
    handle: ApplicationUpdateHandle,
    download_status: UpdateDownloadStatus,
    toast: ToastHandle,
    update_shutdown: ApplicationUpdateShutdown,
) -> Element {
    let version = update.version.clone();

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
                {download_update_button(&update, handle, &download_status, toast, update_shutdown)}
            }
            {download_status_panel(&download_status)}
        }
    }
}

fn download_update_button(
    update: &AvailableUpdate,
    handle: ApplicationUpdateHandle,
    download_status: &UpdateDownloadStatus,
    toast: ToastHandle,
    update_shutdown: ApplicationUpdateShutdown,
) -> Element {
    let version = update.version.clone();
    let has_asset = update.download_asset.is_some();
    let is_downloading = matches!(download_status, UpdateDownloadStatus::Downloading { .. });
    let is_downloaded = matches!(
        download_status,
        UpdateDownloadStatus::Downloaded {
            version: downloaded_version,
            ..
        } if downloaded_version == &version
    );
    let disabled = !has_asset || is_downloading;

    rsx! {
        button {
            r#type: "button",
            disabled,
            class: download_button_class(disabled),
            onclick: move |_| {
                info!(
                    update_version = %version,
                    downloaded = is_downloaded,
                    "application update primary action requested from settings"
                );
                if is_downloaded {
                    if handle.install_downloaded_update() {
                        toast.info("Запускаем установщик обновления.");
                        update_shutdown.close_after_update_started();
                    }
                } else {
                    handle.download_update();
                    toast.info("Начинаем скачивание обновления.");
                }
            },
            if is_downloading {
                "Скачиваем..."
            } else if is_downloaded {
                "Установить"
            } else if has_asset {
                "Скачать обновление"
            } else {
                "Нет установщика"
            }
        }
    }
}

fn download_status_panel(status: &UpdateDownloadStatus) -> Element {
    match status {
        UpdateDownloadStatus::Idle => rsx! {},
        UpdateDownloadStatus::Downloading { version, progress } => rsx! {
            {download_progress_panel(version, *progress)}
        },
        UpdateDownloadStatus::Downloaded { version, file } => rsx! {
            div { class: "mt-3 rounded-xl border border-emerald-400/20 bg-emerald-500/10 px-3 py-2",
                p { class: "text-[12px] font-semibold text-emerald-100", "Обновление {version} скачано" }
                p { class: "mt-1 break-words text-[12px] leading-5 text-emerald-200/75",
                    "Файл: {file.file_name}. Путь: {file.path}"
                }
            }
        },
        UpdateDownloadStatus::Failed { message, .. } => rsx! {
            div { class: "mt-3 rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2",
                p { class: "text-[12px] font-semibold text-red-100", "Не удалось скачать обновление" }
                p { class: "mt-1 text-[12px] leading-5 text-red-200/80", "{message}" }
            }
        },
    }
}

fn download_progress_panel(version: &str, progress: UpdateDownloadProgress) -> Element {
    let percentage = progress_percentage(progress);
    let progress_width = percentage.unwrap_or(100.0);
    let progress_style = format!("width: {progress_width:.1}%;");
    let downloaded = format_bytes(progress.downloaded_bytes);
    let total = progress
        .total_bytes
        .map(format_bytes)
        .unwrap_or_else(|| "неизвестно".to_owned());
    let speed = format_speed(progress.bytes_per_second);

    rsx! {
        div { class: "mt-3 rounded-xl border border-blue-400/20 bg-blue-500/10 px-3 py-2",
            div { class: "flex items-center justify-between gap-3 text-[12px] font-semibold text-blue-100",
                span { "Скачиваем обновление {version}" }
                span {
                    if let Some(percentage) = percentage {
                        "{percentage:.0}%"
                    } else {
                        "{downloaded}"
                    }
                }
            }
            div { class: "mt-2 h-2 w-full overflow-hidden rounded-full bg-zinc-900",
                div { class: "h-full rounded-full bg-blue-400", style: "{progress_style}" }
            }
            p { class: "mt-2 text-[12px] leading-5 text-blue-100/75",
                "{downloaded} из {total} · {speed}"
            }
        }
    }
}

fn progress_percentage(progress: UpdateDownloadProgress) -> Option<f64> {
    let total_bytes = progress.total_bytes?;
    if total_bytes == 0 {
        return None;
    }

    Some(((progress.downloaded_bytes as f64 / total_bytes as f64) * 100.0).clamp(0.0, 100.0))
}

fn format_speed(bytes_per_second: u64) -> String {
    if bytes_per_second == 0 {
        return "скорость считается".to_owned();
    }

    format!("{}/с", format_bytes(bytes_per_second))
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;

    let bytes = bytes as f64;
    if bytes >= GIB {
        return format!("{:.1} ГБ", bytes / GIB);
    }
    if bytes >= MIB {
        return format!("{:.1} МБ", bytes / MIB);
    }
    if bytes >= KIB {
        return format!("{:.1} КБ", bytes / KIB);
    }

    format!("{bytes:.0} Б")
}

fn check_button_class(is_checking: bool) -> &'static str {
    if is_checking {
        "flex h-10 w-full shrink-0 cursor-wait items-center justify-center rounded-xl border border-blue-400/25 bg-blue-500/10 px-3 text-[12px] font-semibold text-blue-100 disabled:opacity-70 sm:h-9 sm:w-auto"
    } else {
        "flex h-10 w-full shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-semibold text-zinc-200 transition hover:border-zinc-700 hover:bg-zinc-900 sm:h-9 sm:w-auto"
    }
}

fn download_button_class(disabled: bool) -> &'static str {
    if disabled {
        "flex h-9 cursor-not-allowed items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/70 px-3 text-[12px] font-semibold text-zinc-500"
    } else {
        "flex h-9 items-center justify-center rounded-xl border border-blue-400/25 bg-blue-500/10 px-3 text-[12px] font-semibold text-blue-100 transition hover:border-blue-400/40 hover:bg-blue-500/15"
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

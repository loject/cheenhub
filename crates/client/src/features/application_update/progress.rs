//! Видимый на всех экранах прогресс скачивания и установки обновления.

use dioxus::prelude::*;

use super::{ApplicationUpdateHandle, UpdateDownloadStatus};

/// Показывает ход текущего обновления независимо от открытого экрана.
#[component]
pub(super) fn ApplicationUpdateProgress() -> Element {
    let handle = use_context::<ApplicationUpdateHandle>();
    let (title, detail, percentage) = match handle.download_status() {
        UpdateDownloadStatus::Downloading { version, progress } => {
            let percentage = progress
                .total_bytes
                .filter(|total| *total > 0)
                .map(|total| {
                    (progress.downloaded_bytes as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
                });
            let downloaded = format_bytes(progress.downloaded_bytes);
            let size = match progress.total_bytes.filter(|total| *total > 0) {
                Some(total) => format!("{downloaded} из {}", format_bytes(total)),
                None => downloaded,
            };
            let detail = if progress.bytes_per_second > 0 {
                format!("{size} · {}/с", format_bytes(progress.bytes_per_second))
            } else {
                size
            };
            (
                format!("Скачиваем обновление {version}"),
                detail,
                percentage,
            )
        }
        UpdateDownloadStatus::Installing {
            version,
            percentage,
        } => (
            format!("Устанавливаем обновление {version}"),
            if percentage.is_some() {
                "Применяем обновление. После установки CheenHub перезапустится."
            } else {
                "Подготавливаем установку. Подтвердите запрос системы, если он появится."
            }
            .to_owned(),
            percentage.map(f64::from),
        ),
        _ => return rsx! {},
    };
    let fill_class = if percentage.is_some() {
        "application-update-progress-fill"
    } else {
        "application-update-progress-fill application-update-progress-indeterminate"
    };
    let style = format!("width: {}%;", percentage.unwrap_or(35.0));

    rsx! {
        aside { class: "application-update-progress-card", aria_label: "Ход обновления CheenHub",
            div { class: "flex items-center justify-between gap-3 text-[12px] font-semibold text-blue-100",
                span { "{title}" }
                if let Some(percentage) = percentage { span { "{percentage:.0}%" } }
            }
            div {
                class: "application-update-progress-track",
                role: "progressbar",
                aria_label: "{title}",
                aria_valuemin: "0",
                aria_valuemax: "100",
                aria_valuenow: percentage.map(|value| format!("{value:.0}")),
                div { class: fill_class, style }
            }
            p { class: "mt-2 text-[12px] leading-5 text-blue-100/75", "{detail}" }
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    let value = bytes as f64;
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} ГБ", value / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} МБ", value / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} КБ", value / 1024.0)
    } else {
        format!("{bytes} Б")
    }
}

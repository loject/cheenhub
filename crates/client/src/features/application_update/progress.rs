//! Представление прогресса обновления для toast-уведомления.

use super::UpdateDownloadStatus;
use crate::features::toast::UpdateToastProgress;

/// Преобразует активный этап обновления в данные уведомления.
///
/// Для завершённых и неактивных операций возвращает `None`.
pub(super) fn toast_progress(status: UpdateDownloadStatus) -> Option<UpdateToastProgress> {
    let (title, detail, percentage) = match status {
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
            percentage.map(|value| f64::from(value.min(100))),
        ),
        _ => return None,
    };
    Some(UpdateToastProgress {
        title,
        detail,
        percentage,
    })
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

#[cfg(test)]
mod tests;

//! Проверки отображения известного и неопределённого прогресса обновления.

use super::*;
use crate::features::application_update::UpdateDownloadProgress;

#[test]
fn download_progress_reports_fraction_size_and_speed() {
    let status = UpdateDownloadStatus::Downloading {
        version: "2".into(),
        progress: UpdateDownloadProgress {
            downloaded_bytes: 1024,
            total_bytes: Some(4096),
            bytes_per_second: 512,
        },
    };

    let progress = toast_progress(status).unwrap();

    assert_eq!(progress.percentage, Some(25.0));
    assert!(progress.detail.contains("1.0 КБ из 4.0 КБ"));
    assert!(progress.detail.contains("512 Б/с"));
}

#[test]
fn unknown_or_zero_download_size_keeps_progress_indeterminate() {
    for total_bytes in [None, Some(0)] {
        let status = UpdateDownloadStatus::Downloading {
            version: "2".into(),
            progress: UpdateDownloadProgress {
                downloaded_bytes: 1024,
                total_bytes,
                bytes_per_second: 0,
            },
        };

        let progress = toast_progress(status).unwrap();

        assert_eq!(progress.percentage, None, "total_bytes={total_bytes:?}");
        assert!(!progress.detail.contains("из"));
    }
}

#[test]
fn install_preparation_waits_for_reported_percentage() {
    let progress = toast_progress(UpdateDownloadStatus::Installing {
        version: "2".into(),
        percentage: None,
    })
    .unwrap();

    assert_eq!(progress.percentage, None);
    assert!(progress.detail.contains("Подтвердите запрос системы"));
}

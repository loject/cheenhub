//! Платформенное скачивание установщиков обновления.

use super::types::{UpdateDownloadAsset, UpdateDownloadProgress, UpdateDownloadStatus};

mod native;
mod unsupported;
mod web;

pub(crate) use native::{
    download_update_asset, install_downloaded_update, opens_download_externally,
    primary_action_presentation, select_update_asset,
};

/// HTTP User-Agent для запросов к GitHub Releases.
pub(super) const USER_AGENT: &str = concat!("CheenHub/", env!("CARGO_PKG_VERSION"));

/// Результат платформенного выбора asset'а из GitHub Release.
pub(crate) type SelectedUpdateAsset = Option<UpdateDownloadAsset>;

/// Возвращает начальное состояние платформенной загрузки.
pub(crate) fn initial_download_status(
    version: String,
    asset: &UpdateDownloadAsset,
) -> UpdateDownloadStatus {
    if opens_download_externally() {
        return UpdateDownloadStatus::OpeningExternal { version };
    }

    UpdateDownloadStatus::Downloading {
        version,
        progress: UpdateDownloadProgress {
            downloaded_bytes: 0,
            total_bytes: (asset.size_bytes > 0).then_some(asset.size_bytes),
            bytes_per_second: 0,
        },
    }
}

/// Представление основного действия с доступным обновлением.
#[derive(Clone, Copy)]
pub(crate) struct PrimaryUpdateActionPresentation {
    /// Текст кнопки.
    pub(crate) label: &'static str,
    /// Нужно ли блокировать кнопку.
    pub(crate) disabled: bool,
    /// Запускает ли действие установщик уже скачанного файла.
    pub(crate) installs_downloaded: bool,
    /// Сообщение после запроса действия пользователем.
    pub(crate) requested_message: &'static str,
}

fn unavailable_action_presentation() -> PrimaryUpdateActionPresentation {
    PrimaryUpdateActionPresentation {
        label: "Нет установщика",
        disabled: true,
        installs_downloaded: false,
        requested_message: "Для этой платформы нет установщика.",
    }
}

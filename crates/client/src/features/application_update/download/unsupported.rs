//! Запасная реализация скачивания обновлений.
#![allow(dead_code)]

use super::{
    PrimaryUpdateActionPresentation, SelectedUpdateAsset, unavailable_action_presentation,
};
use crate::features::application_update::types::UpdateDownloadOutcome;
use crate::features::application_update::{
    AvailableUpdate, DownloadedUpdate, UpdateDownloadAsset, UpdateDownloadProgress,
    UpdateDownloadStatus,
};

pub(crate) fn select_update_asset(_assets: &[UpdateDownloadAsset]) -> SelectedUpdateAsset {
    None
}

/// Сообщает, что неизвестная платформа не открывает внешнюю загрузку.
pub(crate) const fn opens_download_externally() -> bool {
    false
}

/// Возвращает недоступное представление загрузки для неизвестной платформы.
pub(crate) fn primary_action_presentation(
    _update: &AvailableUpdate,
    _download_status: &UpdateDownloadStatus,
) -> PrimaryUpdateActionPresentation {
    unavailable_action_presentation()
}

pub(crate) async fn download_update_asset(
    _asset: UpdateDownloadAsset,
    _on_progress: impl FnMut(UpdateDownloadProgress) + 'static,
) -> Result<UpdateDownloadOutcome, String> {
    Err("Скачивание обновления недоступно на этой платформе.".to_owned())
}

pub(crate) fn install_downloaded_update(
    _version: &str,
    _file: &DownloadedUpdate,
) -> Result<(), String> {
    Err("Установка обновления недоступна на этой платформе.".to_owned())
}

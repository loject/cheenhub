//! Web-реализация скачивания обновлений.
#![cfg_attr(not(target_family = "wasm"), allow(dead_code, unused_imports))]

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

/// Сообщает, что web-клиент не открывает загрузку через native bridge.
pub(crate) const fn opens_download_externally() -> bool {
    false
}

/// Возвращает недоступное представление загрузки для web-клиента.
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
    Err(
        "Скачивание установщика из web-клиента пока недоступно. Откройте релиз на GitHub."
            .to_owned(),
    )
}

pub(crate) fn install_downloaded_update(
    _version: &str,
    _file: &DownloadedUpdate,
) -> Result<(), String> {
    Err("Установка обновления из web-клиента пока недоступна.".to_owned())
}

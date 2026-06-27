//! Запасная реализация скачивания обновлений.
#![allow(dead_code)]

use super::SelectedUpdateAsset;
use crate::features::application_update::{
    DownloadedUpdate, UpdateDownloadAsset, UpdateDownloadProgress,
};

pub(crate) fn select_update_asset(_assets: &[UpdateDownloadAsset]) -> SelectedUpdateAsset {
    None
}

pub(crate) async fn download_update_asset(
    _asset: UpdateDownloadAsset,
    _on_progress: impl FnMut(UpdateDownloadProgress) + 'static,
) -> Result<DownloadedUpdate, String> {
    Err("Скачивание обновления недоступно на этой платформе.".to_owned())
}

pub(crate) fn install_downloaded_update(
    _version: &str,
    _file: &DownloadedUpdate,
) -> Result<(), String> {
    Err("Установка обновления недоступна на этой платформе.".to_owned())
}

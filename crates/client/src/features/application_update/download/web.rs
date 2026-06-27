//! Web-реализация скачивания обновлений.
#![cfg_attr(not(target_family = "wasm"), allow(dead_code, unused_imports))]

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
    Err(
        "Скачивание установщика из web-клиента пока недоступно. Откройте релиз на GitHub."
            .to_owned(),
    )
}

pub(crate) fn install_downloaded_update(_file: &DownloadedUpdate) -> Result<(), String> {
    Err("Установка обновления из web-клиента пока недоступна.".to_owned())
}

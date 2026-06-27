//! Платформенное скачивание установщиков обновления.

use super::types::UpdateDownloadAsset;

mod native;
mod unsupported;
mod web;

pub(crate) use native::{download_update_asset, install_downloaded_update, select_update_asset};

/// HTTP User-Agent для запросов к GitHub Releases.
pub(super) const USER_AGENT: &str = concat!("CheenHub/", env!("CARGO_PKG_VERSION"));

/// Результат платформенного выбора asset'а из GitHub Release.
pub(crate) type SelectedUpdateAsset = Option<UpdateDownloadAsset>;

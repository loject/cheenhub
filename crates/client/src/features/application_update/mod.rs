//! UI-состояние обновлений клиентского приложения.

mod api;
mod download;
mod handle;
mod provider;
mod storage;
mod types;

pub(crate) use handle::{ApplicationUpdateHandle, UpdateUiStatus, now_epoch_seconds};
pub(crate) use provider::ApplicationUpdateProvider;
pub(crate) use types::{
    AvailableUpdate, DownloadedUpdate, UpdateDownloadAsset, UpdateDownloadProgress,
    UpdateDownloadStatus,
};

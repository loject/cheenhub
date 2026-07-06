//! UI-состояние обновлений клиентского приложения.

mod api;
mod download;
mod effects;
mod handle;
mod notifications;
mod provider;
mod shutdown;
mod storage;
mod types;

pub(crate) use handle::{ApplicationUpdateHandle, UpdateUiStatus, now_epoch_seconds};
pub(crate) use provider::ApplicationUpdateProvider;
pub(crate) use shutdown::{ApplicationUpdateShutdown, use_application_update_shutdown};
pub(crate) use types::{
    AvailableUpdate, DownloadedUpdate, UpdateDownloadAsset, UpdateDownloadProgress,
    UpdateDownloadStatus,
};

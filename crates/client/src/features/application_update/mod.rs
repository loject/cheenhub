//! UI-состояние обновлений клиентского приложения.

mod api;
mod handle;
mod provider;
mod storage;

pub(crate) use handle::{
    ApplicationUpdateHandle, AvailableUpdate, UpdateUiStatus, now_epoch_seconds,
};
pub(crate) use provider::ApplicationUpdateProvider;

//! Работа с версиями и историей релизов приложения.

use super::ApplicationUpdateHandle;
use crate::features::application_update::{AvailableUpdate, api};

impl ApplicationUpdateHandle {
    /// Возвращает текущую версию клиентского приложения.
    pub(crate) fn current_version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Загружает список стабильных релизов старше установленной версии.
    pub(crate) async fn previous_releases(&self) -> Result<Vec<AvailableUpdate>, String> {
        api::list_previous_releases(self.current_version()).await
    }
}

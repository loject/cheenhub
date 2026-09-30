//! Запуск установки и доставка прогресса из платформенного рабочего потока.

use dioxus::dioxus_core::spawn_forever;
use dioxus::prelude::*;
use futures_util::StreamExt;

use super::ApplicationUpdateHandle;
use crate::features::application_update::{UpdateDownloadStatus, download};

impl ApplicationUpdateHandle {
    /// Запускает установщик уже скачанного обновления.
    pub(crate) fn install_downloaded_update(&self) -> bool {
        let UpdateDownloadStatus::Downloaded { version, file } = (self.state)().download_status
        else {
            warn!("application update install requested without downloaded update");
            return false;
        };

        let (progress, mut updates) = futures_channel::mpsc::unbounded();
        match download::install_downloaded_update(&version, &file, progress) {
            Ok(()) => {
                info!(
                    update_version = %version,
                    update_path = %file.path,
                    "application update installer started"
                );
                let mut state = self.state;
                state.with_mut(|state| {
                    state.download_status = UpdateDownloadStatus::Installing {
                        version: version.clone(),
                        percentage: None,
                    };
                });
                spawn_forever(async move {
                    while let Some(update) = updates.next().await {
                        state.with_mut(|state| {
                            state.download_status = match update {
                                Ok(percentage) => UpdateDownloadStatus::Installing {
                                    version: version.clone(),
                                    percentage: Some(percentage),
                                },
                                Err(message) => UpdateDownloadStatus::Failed {
                                    version: version.clone(),
                                    message,
                                },
                            };
                        });
                    }
                });
                true
            }
            Err(message) => {
                warn!(
                    update_version = %version,
                    %message,
                    "application update installer failed to start"
                );
                let mut state = self.state;
                state.with_mut(|state| {
                    state.download_status = UpdateDownloadStatus::Failed { version, message };
                });
                false
            }
        }
    }
}

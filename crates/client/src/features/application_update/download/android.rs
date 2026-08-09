//! Android-реализация открытия APK обновления во внешнем браузере.

use dioxus::prelude::*;
use futures_channel::oneshot;
use jni::objects::JValue;

use super::super::{PrimaryUpdateActionPresentation, unavailable_action_presentation};
use crate::features::application_update::types::UpdateDownloadOutcome;
use crate::features::application_update::{
    AvailableUpdate, DownloadedUpdate, UpdateDownloadAsset, UpdateDownloadProgress,
    UpdateDownloadStatus,
};

const PREFERRED_SUFFIXES: &[&str] = &["android.apk"];

/// Выбирает APK текущего Android-релиза.
pub(crate) fn select_update_asset(assets: &[UpdateDownloadAsset]) -> Option<UpdateDownloadAsset> {
    for suffix in PREFERRED_SUFFIXES {
        if let Some(asset) = assets
            .iter()
            .find(|asset| asset.name.ends_with(suffix))
            .cloned()
        {
            return Some(asset);
        }
    }

    None
}

/// Сообщает, что Android открывает загрузку во внешнем браузере.
pub(crate) const fn opens_download_externally() -> bool {
    true
}

/// Возвращает Android-представление основного действия обновления.
pub(crate) fn primary_action_presentation(
    update: &AvailableUpdate,
    download_status: &UpdateDownloadStatus,
) -> PrimaryUpdateActionPresentation {
    if update.download_asset.is_none() {
        return unavailable_action_presentation();
    }

    match download_status {
        UpdateDownloadStatus::OpeningExternal { version } if version == &update.version => {
            PrimaryUpdateActionPresentation {
                label: "Открываем браузер...",
                disabled: true,
                installs_downloaded: false,
                requested_message: "Открываем загрузку APK во внешнем браузере.",
            }
        }
        UpdateDownloadStatus::OpenedExternally { version } if version == &update.version => {
            PrimaryUpdateActionPresentation {
                label: "Открыть APK снова",
                disabled: false,
                installs_downloaded: false,
                requested_message: "Снова открываем загрузку APK во внешнем браузере.",
            }
        }
        _ => PrimaryUpdateActionPresentation {
            label: "Скачать APK в браузере",
            disabled: false,
            installs_downloaded: false,
            requested_message: "Открываем загрузку APK во внешнем браузере.",
        },
    }
}

/// Открывает прямую ссылку на APK во внешнем системном браузере.
pub(crate) async fn download_update_asset(
    asset: UpdateDownloadAsset,
    _on_progress: impl FnMut(UpdateDownloadProgress) + 'static,
) -> Result<UpdateDownloadOutcome, String> {
    info!(
        asset_name = %asset.name,
        "opening Android application update asset in external browser"
    );
    open_external_browser(asset.download_url).await?;
    info!(
        asset_name = %asset.name,
        "Android application update asset opened in external browser"
    );
    Ok(UpdateDownloadOutcome {
        downloaded_file: None,
    })
}

/// Отклоняет недоступную на Android установку локального файла из приложения.
pub(crate) fn install_downloaded_update(
    _version: &str,
    _file: &DownloadedUpdate,
) -> Result<(), String> {
    Err("На Android APK скачивается и устанавливается через системный браузер.".to_owned())
}

async fn open_external_browser(download_url: String) -> Result<(), String> {
    let (sender, receiver) = oneshot::channel();
    wry::prelude::dispatch(move |env, activity, _| {
        let result = env
            .new_string(download_url)
            .and_then(|download_url| {
                env.call_method(
                    activity,
                    "openCheenHubUpdateDownload",
                    "(Ljava/lang/String;)Z",
                    &[JValue::Object(&download_url)],
                )
            })
            .and_then(|result| result.z())
            .map_err(|error| format!("Не удалось передать ссылку Android: {error}"))
            .and_then(|opened| {
                opened
                    .then_some(())
                    .ok_or_else(|| "Android не смог открыть браузер для скачивания APK.".to_owned())
            });
        let _ = sender.send(result);
    });

    receiver
        .await
        .map_err(|_| "Android закрыл запрос открытия браузера.".to_owned())?
}

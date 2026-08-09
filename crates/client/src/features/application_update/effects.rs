//! Эффекты проверки обновлений приложения и связанные toast-уведомления.

use std::time::Duration;

use dioxus::prelude::*;

use super::handle::{
    ApplicationUpdateHandle, UpdateDeferralDelay, UpdateUiStatus, now_epoch_seconds,
};
use super::notifications::application_update_notifications_enabled;
use super::primary_action_presentation;
use super::shutdown::{ApplicationUpdateShutdown, use_application_update_shutdown};
use super::types::{AvailableUpdate, UpdateDownloadStatus};
use crate::features::runtime::sleep_duration;
use crate::features::toast::{
    ToastHandle, UpdateAvailableToast, UpdateAvailableToastActions, UpdateAvailableToastContent,
    UpdateToastDeferralOption,
};

/// Запускает проверку обновлений и синхронизирует ее результат с toast-уведомлениями.
#[component]
pub(super) fn ApplicationUpdateEffects(children: Element) -> Element {
    let handle = use_context::<ApplicationUpdateHandle>();
    let toast = use_context::<ToastHandle>();
    let update_shutdown = use_application_update_shutdown();
    let mut auto_check_started = use_signal(|| false);
    let mut scheduled_deferral = use_signal(|| None::<(String, u64)>);
    let mut shown_notification_version = use_signal(|| None::<String>);
    let mut reported_download_status = use_signal(|| None::<String>);

    use_effect(move || {
        if auto_check_started() {
            return;
        }

        auto_check_started.set(true);
        if !application_update_notifications_enabled() {
            debug!("automatic application update check is disabled on this platform");
            return;
        }

        handle.check_now();
    });

    use_effect(move || match handle.ui_status() {
        UpdateUiStatus::Deferred {
            update,
            until_epoch_seconds,
        } => {
            let scheduled = Some((update.version.clone(), until_epoch_seconds));
            if scheduled_deferral() == scheduled {
                return;
            }

            scheduled_deferral.set(scheduled);
            spawn(async move {
                let seconds = until_epoch_seconds.saturating_sub(now_epoch_seconds());
                sleep_duration(Duration::from_secs(seconds)).await;
                handle.release_deferral_if_due(&update.version, until_epoch_seconds);
            });
        }
        _ => {
            if scheduled_deferral().is_some() {
                scheduled_deferral.set(None);
            }
        }
    });

    use_effect(move || {
        if !application_update_notifications_enabled() {
            if shown_notification_version().is_some() {
                shown_notification_version.set(None);
            }
            return;
        }

        let Some(update) = handle.should_show_notification() else {
            if shown_notification_version().is_some() {
                shown_notification_version.set(None);
            }
            return;
        };

        let download_status = handle.download_status();
        let notification_key = update_notification_key(&update.version, &download_status);
        if shown_notification_version().as_deref() == Some(notification_key.as_str()) {
            return;
        }

        shown_notification_version.set(Some(notification_key));
        toast.update_available(update_available_toast(
            update,
            handle,
            toast,
            download_status,
            update_shutdown.clone(),
        ));
    });

    use_effect(move || match handle.download_status() {
        UpdateDownloadStatus::Idle
        | UpdateDownloadStatus::Downloading { .. }
        | UpdateDownloadStatus::OpeningExternal { .. } => {
            if reported_download_status().is_some() {
                reported_download_status.set(None);
            }
        }
        UpdateDownloadStatus::Downloaded { version, file } => {
            let key = format!("downloaded:{version}:{}", file.path);
            if reported_download_status().as_deref() == Some(key.as_str()) {
                return;
            }

            reported_download_status.set(Some(key));
            toast.success(format!("Обновление {version} скачано: {}.", file.file_name));
        }
        UpdateDownloadStatus::OpenedExternally { version } => {
            let key = format!("opened-externally:{version}");
            if reported_download_status().as_deref() == Some(key.as_str()) {
                return;
            }

            reported_download_status.set(Some(key));
            toast.success(format!(
                "Загрузка APK {version} открыта во внешнем браузере."
            ));
        }
        UpdateDownloadStatus::Failed { version, message } => {
            let key = format!("failed:{version}:{message}");
            if reported_download_status().as_deref() == Some(key.as_str()) {
                return;
            }

            reported_download_status.set(Some(key));
            toast.error(message);
        }
    });

    rsx! {
        {children}
    }
}

fn update_available_toast(
    update: AvailableUpdate,
    handle: ApplicationUpdateHandle,
    toast: ToastHandle,
    download_status: UpdateDownloadStatus,
    update_shutdown: ApplicationUpdateShutdown,
) -> UpdateAvailableToast {
    let action_version = update.version.clone();
    let primary_state = primary_action_presentation(&update, &download_status);
    let download_handle = handle;
    let download_toast = toast;
    let defer_toast = toast;
    let shutdown_after_update_start = update_shutdown;
    let quick_dismiss_handle = handle;
    let defer_handle = handle;

    UpdateAvailableToast::new(
        UpdateAvailableToastContent::new(
            handle.current_version(),
            update.version,
            update.title,
            primary_state.label,
            primary_state.disabled,
            UpdateDeferralDelay::all()
                .into_iter()
                .map(|delay| UpdateToastDeferralOption::new(delay.value(), delay.label()))
                .collect(),
            UpdateDeferralDelay::Tomorrow.value(),
        ),
        UpdateAvailableToastActions::new(
            move || {
                info!(
                    update_version = %action_version,
                    installs_downloaded = primary_state.installs_downloaded,
                    "application update primary action requested from notification"
                );
                if primary_state.installs_downloaded {
                    if download_handle.install_downloaded_update() {
                        download_toast.info(primary_state.requested_message);
                        shutdown_after_update_start.close_after_update_started();
                    }
                } else {
                    download_handle.download_update();
                    download_toast.info(primary_state.requested_message);
                }
            },
            move || quick_dismiss_handle.dismiss_update_for_five_minutes(),
            move |delay_value| {
                let delay = UpdateDeferralDelay::from_value(&delay_value);
                defer_handle.defer_update(delay);
                defer_toast.info(format!(
                    "Напомним об обновлении: {}.",
                    delay.label().to_lowercase()
                ));
            },
        ),
    )
}

fn update_notification_key(version: &str, download_status: &UpdateDownloadStatus) -> String {
    let phase = match download_status {
        UpdateDownloadStatus::Downloading {
            version: download_version,
            ..
        } if download_version == version => "downloading",
        UpdateDownloadStatus::Downloaded {
            version: download_version,
            ..
        } if download_version == version => "downloaded",
        UpdateDownloadStatus::OpeningExternal {
            version: download_version,
        } if download_version == version => "opening-external",
        UpdateDownloadStatus::OpenedExternally {
            version: download_version,
        } if download_version == version => "opened-externally",
        _ => "available",
    };

    format!("{version}:{phase}")
}

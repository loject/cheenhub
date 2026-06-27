//! Провайдер пользовательского состояния обновлений приложения.

use std::time::Duration;

use dioxus::prelude::*;

use super::handle::{
    ApplicationUpdateHandle, ApplicationUpdateState, AvailableUpdate, UpdateDeferralDelay,
    UpdateUiStatus, now_epoch_seconds,
};
use crate::features::runtime::sleep_duration;
use crate::features::toast::{
    ToastHandle, UpdateAvailableToast, UpdateAvailableToastActions, UpdateAvailableToastContent,
    UpdateToastDeferralOption,
};

/// Предоставляет состояние проверки обновлений всем экранам клиента.
#[component]
pub(crate) fn ApplicationUpdateProvider(children: Element) -> Element {
    let state = use_signal(ApplicationUpdateState::default);
    let handle = ApplicationUpdateHandle::new(state);
    let toast = use_context::<ToastHandle>();
    let mut auto_check_started = use_signal(|| false);
    let mut scheduled_deferral = use_signal(|| None::<(String, u64)>);
    let mut shown_notification_version = use_signal(|| None::<String>);
    use_context_provider(move || handle);

    use_effect(move || {
        if auto_check_started() {
            return;
        }

        auto_check_started.set(true);
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
        let Some(update) = handle.should_show_notification() else {
            if shown_notification_version().is_some() {
                shown_notification_version.set(None);
            }
            return;
        };

        if shown_notification_version().as_deref() == Some(update.version.as_str()) {
            return;
        }

        shown_notification_version.set(Some(update.version.clone()));
        toast.update_available(update_available_toast(update, handle, toast));
    });

    rsx! {
        {children}
    }
}

fn update_available_toast(
    update: AvailableUpdate,
    handle: ApplicationUpdateHandle,
    toast: ToastHandle,
) -> UpdateAvailableToast {
    let install_update_version = update.version.clone();
    let release_url = update.release_url.clone();
    let install_toast = toast;
    let defer_toast = toast;
    let quick_dismiss_handle = handle;
    let defer_handle = handle;

    UpdateAvailableToast::new(
        UpdateAvailableToastContent::new(
            handle.current_version(),
            update.version,
            update.title,
            UpdateDeferralDelay::all()
                .into_iter()
                .map(|delay| UpdateToastDeferralOption::new(delay.value(), delay.label()))
                .collect(),
            UpdateDeferralDelay::Tomorrow.value(),
        ),
        UpdateAvailableToastActions::new(
            move || {
                info!(
                    update_version = %install_update_version,
                    release_url = %release_url,
                    "application update install requested while installer is not implemented"
                );
                install_toast.info("Проверка обновлений уже работает. Скачивание и установка будут подключены отдельным шагом.");
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

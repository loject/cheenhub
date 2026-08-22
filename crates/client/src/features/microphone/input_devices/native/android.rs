//! Перечисление устройств ввода аудио на Android.

use dioxus::prelude::{info, warn};

use super::super::contract::AudioInputDevicesResult;
use crate::features::runtime::android::{AndroidPermission, PermissionResult, android_bridge};

/// Сообщает, что выбор конкретного микрофона выполняет Android audio policy.
pub(crate) async fn enumerate_audio_input_devices() -> AudioInputDevicesResult {
    info!(
        platform = "android",
        management = "system_audio_policy",
        "microphone input device selection is managed by the system"
    );
    AudioInputDevicesResult {
        devices: Some(Vec::new()),
        system_managed: true,
        permission_required: false,
        permission_denied: false,
    }
}

/// Запрашивает runtime-разрешение Android на запись звука.
pub(crate) async fn request_microphone_permission() -> AudioInputDevicesResult {
    info!(
        platform = "android",
        permission = "record_audio",
        "requesting microphone permission"
    );
    let bridge = match android_bridge() {
        Ok(bridge) => bridge,
        Err(error) => {
            warn!(error = %error, "Android microphone permission bridge is unavailable");
            return unavailable_input_devices();
        }
    };
    let (sender, receiver) = futures_channel::oneshot::channel();
    if let Err(error) = bridge.request_permission(
        AndroidPermission::RecordAudio,
        Box::new(move |result| {
            let _ = sender.send(result);
        }),
    ) {
        warn!(error = %error, "failed to request Android microphone permission");
        return unavailable_input_devices();
    }

    match receiver.await {
        Ok(Ok(PermissionResult::Granted)) => {
            info!(
                platform = "android",
                permission = "record_audio",
                granted = true,
                "microphone permission request completed"
            );
            enumerate_audio_input_devices().await
        }
        Ok(Ok(PermissionResult::Denied | PermissionResult::DeniedPermanently)) => {
            info!(
                platform = "android",
                permission = "record_audio",
                granted = false,
                "microphone permission request completed"
            );
            AudioInputDevicesResult {
                devices: None,
                system_managed: false,
                permission_required: false,
                permission_denied: true,
            }
        }
        Ok(Err(error)) => {
            warn!(error = %error, "Android microphone permission request failed");
            unavailable_input_devices()
        }
        Err(error) => {
            warn!(error = %error, "Android microphone permission callback was dropped");
            unavailable_input_devices()
        }
    }
}

fn unavailable_input_devices() -> AudioInputDevicesResult {
    AudioInputDevicesResult {
        devices: None,
        system_managed: false,
        permission_required: false,
        permission_denied: false,
    }
}

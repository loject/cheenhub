//! Загрузка и обновление доступных аудиоустройств в открытых настройках.

use dioxus::prelude::*;

use crate::features::audio_playback::{
    AudioOutputDevice, AudioOutputDevicesResult, AudioPlaybackHandle,
    enumerate_audio_output_devices,
};
use crate::features::microphone::{
    AudioInputDevice, AudioInputDevicesResult, MicrophoneHandle, enumerate_audio_input_devices,
    request_microphone_permission,
};

mod native;

/// Контракт ожидания повторной проверки подключённых устройств.
trait RefreshTimer {
    /// Ждёт следующую проверку либо закрытие области настроек.
    fn wait() -> impl std::future::Future<Output = ()>;
}

/// Обновляет списки до закрытия компонента; жизненным циклом управляет `use_future`.
pub(super) async fn refresh_while_mounted(
    mic: MicrophoneHandle,
    playback: AudioPlaybackHandle,
    mut input_state: Signal<Option<AudioInputDevicesResult>>,
    mut output_state: Signal<Option<AudioOutputDevicesResult>>,
) {
    let mut initial = true;
    loop {
        futures_util::future::join(
            async {
                let result = enumerate_audio_input_devices().await;
                if input_state.peek().as_ref() != Some(&result) {
                    debug!(
                        available = result.devices.is_some(),
                        "sound settings input device list changed"
                    );
                    reconcile_input_devices_result(&mic, &result);
                    input_state.set(Some(result));
                }
                if initial {
                    mic.start_level_preview();
                }
            },
            async {
                let result = enumerate_audio_output_devices().await;
                if output_state.peek().as_ref() != Some(&result) {
                    debug!(
                        available = result.devices.is_some(),
                        "sound settings output device list changed"
                    );
                    reconcile_output_devices_result(&playback, &result);
                    output_state.set(Some(result));
                }
            },
        )
        .await;
        initial = false;
        native::Timer::wait().await;
    }
}

/// Запрашивает разрешение и обновляет оба списка устройств.
pub(super) fn refresh_devices_after_permission(
    mic: MicrophoneHandle,
    playback: AudioPlaybackHandle,
    mut input_devices_state: Signal<Option<AudioInputDevicesResult>>,
    mut output_devices_state: Signal<Option<AudioOutputDevicesResult>>,
    mut requesting_permission: Signal<bool>,
) {
    requesting_permission.set(true);
    input_devices_state.set(None);
    output_devices_state.set(None);

    spawn(async move {
        let input_result = request_microphone_permission().await;
        reconcile_input_devices_result(&mic, &input_result);
        input_devices_state.set(Some(input_result));
        mic.start_level_preview();

        let output_result = enumerate_audio_output_devices().await;
        reconcile_output_devices_result(&playback, &output_result);
        output_devices_state.set(Some(output_result));

        requesting_permission.set(false);
    });
}

/// Обновляет оба списка после успешного запуска preview без повторного захвата микрофона.
pub(super) fn refresh_devices_after_preview_started(
    mic: MicrophoneHandle,
    playback: AudioPlaybackHandle,
    mut input_devices_state: Signal<Option<AudioInputDevicesResult>>,
    mut output_devices_state: Signal<Option<AudioOutputDevicesResult>>,
) {
    spawn(async move {
        debug!("refreshing sound device lists after microphone preview started");
        let (input_result, output_result) = futures_util::future::join(
            enumerate_audio_input_devices(),
            enumerate_audio_output_devices(),
        )
        .await;

        reconcile_input_devices_result(&mic, &input_result);
        input_devices_state.set(Some(input_result));
        reconcile_output_devices_result(&playback, &output_result);
        output_devices_state.set(Some(output_result));
    });
}

/// Повторяет загрузку устройств ввода с индикатором ожидания.
pub(super) fn refresh_input_devices(
    mic: MicrophoneHandle,
    mut input_devices_state: Signal<Option<AudioInputDevicesResult>>,
) {
    input_devices_state.set(None);

    spawn(async move {
        let result = enumerate_audio_input_devices().await;
        reconcile_input_devices_result(&mic, &result);
        input_devices_state.set(Some(result));
        mic.start_level_preview();
    });
}

/// Повторяет загрузку устройств вывода с индикатором ожидания.
pub(super) fn refresh_output_devices(
    playback: AudioPlaybackHandle,
    mut output_devices_state: Signal<Option<AudioOutputDevicesResult>>,
) {
    output_devices_state.set(None);

    spawn(async move {
        let result = enumerate_audio_output_devices().await;
        reconcile_output_devices_result(&playback, &result);
        output_devices_state.set(Some(result));
    });
}

fn reconcile_input_devices_result(mic: &MicrophoneHandle, result: &AudioInputDevicesResult) {
    if result.system_managed && mic.input_device_id().is_some() {
        info!(
            platform = "android",
            management = "system_audio_policy",
            "clearing stored microphone input device preference"
        );
        mic.set_input_device(&AudioInputDevice {
            device_id: String::new(),
            label: String::new(),
        });
    } else if let Some(devices) = result
        .devices
        .as_ref()
        .filter(|devices| !devices.is_empty())
    {
        mic.reconcile_input_devices(devices);
    }
}

fn reconcile_output_devices_result(
    playback: &AudioPlaybackHandle,
    result: &AudioOutputDevicesResult,
) {
    if result.system_managed && playback.output_device_id().is_some() {
        info!(
            platform = "android",
            management = "audio_manager",
            "clearing stored audio output device preference"
        );
        playback.set_output_device(&AudioOutputDevice {
            device_id: String::new(),
            label: String::new(),
        });
    } else if let Some(devices) = result
        .devices
        .as_ref()
        .filter(|devices| !devices.is_empty())
    {
        playback.reconcile_output_devices(devices);
    }
}

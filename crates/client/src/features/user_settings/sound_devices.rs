//! Sound settings device selector helpers.

use dioxus::prelude::*;

use crate::features::audio_playback::{AudioOutputDevice, AudioOutputDevicesResult};
use crate::features::microphone::{AudioInputDevice, AudioInputDevicesResult};

use super::styles::select_class;

pub(super) fn input_device_widget(
    state: Option<AudioInputDevicesResult>,
    requesting_permission: bool,
    selected: Option<String>,
    mut on_change: impl FnMut(AudioInputDevice) + 'static,
    on_request_permission: impl FnMut(Event<MouseData>) + 'static,
    on_retry: impl FnMut(Event<MouseData>) + 'static,
) -> Element {
    match state {
        None => loading_devices(),

        Some(AudioInputDevicesResult::Available(devices)) => rsx! {
            {
                let change_devices = devices.clone();
                rsx! {
                    select {
                        value: selected.as_deref().unwrap_or_default(),
                        oninput: move |event| {
                            let device_id = event.value();
                            info!("selected microphone input device in settings ui");
                            let device = change_devices
                                .iter()
                                .find(|device| device.device_id == device_id)
                                .cloned()
                                .unwrap_or_else(|| AudioInputDevice {
                                    device_id,
                                    label: String::new(),
                                });
                            on_change(device);
                        },
                        class: select_class(),
                        option {
                            value: "",
                            selected: selected.as_deref().unwrap_or_default().is_empty(),
                            "Системное устройство по умолчанию"
                        }
                        if selected_input_device_is_unavailable(selected.as_deref(), &devices) {
                            option {
                                value: selected.as_deref().unwrap_or_default(),
                                selected: true,
                                "Сохранённое устройство недоступно"
                            }
                        }
                        for device in devices {
                            option {
                                value: device.device_id.clone(),
                                selected: selected.as_deref() == Some(device.device_id.as_str()),
                                {input_device_display_label(&device)}
                            }
                        }
                    }
                }
            }
        },

        Some(AudioInputDevicesResult::PermissionRequired) => permission_required(
            "🎙",
            "Для выбора устройства разрешите доступ к микрофону.",
            requesting_permission,
            on_request_permission,
        ),

        Some(AudioInputDevicesResult::PermissionDenied) => rsx! {
            div { class: "flex items-start gap-2 rounded-xl border border-red-500/20 bg-red-500/8 px-3 py-2.5",
                span { class: "mt-px shrink-0 text-red-400", "⊘" }
                p { class: "text-[12px] leading-5 text-red-300",
                    "Доступ к микрофону запрещён. Разрешите его в настройках системы или браузера и обновите список."
                }
            }
        },

        Some(AudioInputDevicesResult::NoDevices) => no_devices(
            "Устройства ввода не обнаружены. Подключите микрофон и повторите.",
            on_retry,
        ),

        Some(AudioInputDevicesResult::NotSupported) => {
            not_supported("Текущая платформа не поддерживает выбор устройств аудиовхода.")
        }
    }
}

pub(super) fn output_device_widget(
    state: Option<AudioOutputDevicesResult>,
    requesting_permission: bool,
    selected: Option<String>,
    mut on_change: impl FnMut(AudioOutputDevice) + 'static,
    on_request_permission: impl FnMut(Event<MouseData>) + 'static,
    on_retry: impl FnMut(Event<MouseData>) + 'static,
) -> Element {
    match state {
        None => loading_devices(),

        Some(AudioOutputDevicesResult::Available(devices)) => rsx! {
            {
                let change_devices = devices.clone();
                rsx! {
                    select {
                        value: selected.as_deref().unwrap_or_default(),
                        oninput: move |event| {
                            let device_id = event.value();
                            info!("selected audio output device in settings ui");
                            let device = change_devices
                                .iter()
                                .find(|device| device.device_id == device_id)
                                .cloned()
                                .unwrap_or_else(|| AudioOutputDevice {
                                    device_id,
                                    label: String::new(),
                                });
                            on_change(device);
                        },
                        class: select_class(),
                        option {
                            value: "",
                            selected: selected.as_deref().unwrap_or_default().is_empty(),
                            "Системное устройство по умолчанию"
                        }
                        if selected_output_device_is_unavailable(selected.as_deref(), &devices) {
                            option {
                                value: selected.as_deref().unwrap_or_default(),
                                selected: true,
                                "Сохранённое устройство недоступно"
                            }
                        }
                        for device in devices {
                            option {
                                value: device.device_id.clone(),
                                selected: selected.as_deref() == Some(device.device_id.as_str()),
                                {output_device_display_label(&device)}
                            }
                        }
                    }
                }
            }
        },

        Some(AudioOutputDevicesResult::PermissionRequired) => permission_required(
            "◉",
            "Для выбора устройства вывода разрешите доступ к аудио.",
            requesting_permission,
            on_request_permission,
        ),

        Some(AudioOutputDevicesResult::NoDevices) => no_devices(
            "Устройства вывода не обнаружены. Подключите устройство и повторите.",
            on_retry,
        ),

        Some(AudioOutputDevicesResult::NotSupported) => {
            not_supported("Текущая платформа не поддерживает выбор устройств аудиовывода.")
        }
    }
}

fn loading_devices() -> Element {
    rsx! {
        div { class: "flex h-10 items-center gap-2 text-[13px] text-zinc-500",
            span { class: "inline-block h-3 w-3 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-400" }
            "Загрузка устройств…"
        }
    }
}

fn permission_required(
    icon: &'static str,
    message: &'static str,
    requesting_permission: bool,
    on_request_permission: impl FnMut(Event<MouseData>) + 'static,
) -> Element {
    rsx! {
        div { class: "space-y-2",
            div { class: "flex items-start gap-2 rounded-xl border border-blue-500/20 bg-blue-500/8 px-3 py-2.5",
                span { class: "mt-px shrink-0 text-blue-400", "{icon}" }
                p { class: "text-[12px] leading-5 text-blue-300", "{message}" }
            }
            button {
                r#type: "button",
                disabled: requesting_permission,
                onclick: on_request_permission,
                class: "flex h-9 w-full items-center justify-center gap-2 rounded-xl border border-blue-500/30 bg-blue-500/10 px-4 text-[12px] font-medium text-blue-300 transition hover:border-blue-400/50 hover:bg-blue-500/15 disabled:cursor-not-allowed disabled:opacity-50",
                if requesting_permission {
                    span { class: "inline-block h-3 w-3 animate-spin rounded-full border-2 border-blue-500/40 border-t-blue-400" }
                    "Запрос доступа…"
                } else {
                    "Разрешить доступ"
                }
            }
        }
    }
}

fn no_devices(message: &'static str, on_retry: impl FnMut(Event<MouseData>) + 'static) -> Element {
    rsx! {
        div { class: "space-y-2",
            div { class: "flex items-start gap-2 rounded-xl border border-amber-500/20 bg-amber-500/8 px-3 py-2.5",
                span { class: "mt-px shrink-0 text-amber-400", "!" }
                p { class: "text-[12px] leading-5 text-amber-300", "{message}" }
            }
            button {
                r#type: "button",
                onclick: on_retry,
                class: "flex h-9 w-full items-center justify-center rounded-xl border border-zinc-700 bg-zinc-900 px-4 text-[12px] font-medium text-zinc-300 transition hover:border-zinc-500 hover:text-zinc-100",
                "Обновить список"
            }
        }
    }
}

fn not_supported(message: &'static str) -> Element {
    rsx! {
        div { class: "flex items-start gap-2 rounded-xl border border-zinc-700/50 bg-zinc-900/50 px-3 py-2.5",
            span { class: "mt-px shrink-0 text-zinc-500", "⊘" }
            p { class: "text-[12px] leading-5 text-zinc-500", "{message}" }
        }
    }
}

fn selected_input_device_is_unavailable(
    selected: Option<&str>,
    devices: &[AudioInputDevice],
) -> bool {
    selected
        .filter(|selected| !selected.is_empty())
        .is_some_and(|selected| !devices.iter().any(|device| device.device_id == selected))
}

fn selected_output_device_is_unavailable(
    selected: Option<&str>,
    devices: &[AudioOutputDevice],
) -> bool {
    selected
        .filter(|selected| !selected.is_empty())
        .is_some_and(|selected| !devices.iter().any(|device| device.device_id == selected))
}

fn input_device_display_label(device: &AudioInputDevice) -> String {
    if device.label.is_empty() {
        let preview: String = device.device_id.chars().take(8).collect();
        format!("Устройство ({preview}...)")
    } else {
        device.label.clone()
    }
}

fn output_device_display_label(device: &AudioOutputDevice) -> String {
    if device.label.is_empty() {
        let preview: String = device.device_id.chars().take(8).collect();
        format!("Устройство ({preview}...)")
    } else {
        device.label.clone()
    }
}

//! User sound settings section.

use dioxus::prelude::*;

use crate::features::microphone::{
    AudioInputDevice, AudioInputDevicesResult, MicrophoneHandle, enumerate_audio_input_devices,
    request_microphone_permission,
};

use super::styles::{parse_percent, select_class};

#[derive(Clone, Copy, PartialEq)]
enum ActivationMode {
    AlwaysOn,
    VoiceActivation,
}

/// Renders sound input, output, and voice activation controls.
#[component]
pub(crate) fn SoundSettingsSection() -> Element {
    let mic = use_context::<MicrophoneHandle>();

    // Read the stored device preference — this creates a reactive subscription so the
    // select re-renders whenever the stored value changes.
    let selected_device_id = mic.input_device_id();

    let mut devices_state = use_signal(|| Option::<AudioInputDevicesResult>::None);
    let mut requesting_permission = use_signal(|| false);

    let mut output_device = use_signal(|| "SteelSeries Arctis 7".to_owned());
    let mut input_volume = use_signal(|| 75);
    let mut output_volume = use_signal(|| 60);
    let mut activation_mode = use_signal(|| ActivationMode::AlwaysOn);
    let mut activation_level = use_signal(|| 45);
    let live_level = 58;

    // Enumerate real devices once on mount. Only auto-selects the first device when
    // no valid preference is already stored in the microphone handle context.
    let mic_effect = mic.clone();
    use_effect(move || {
        let mic = mic_effect.clone();
        spawn(async move {
            let result = enumerate_audio_input_devices().await;
            if let AudioInputDevicesResult::Available(ref devices) = result {
                let existing_id = mic.input_device_id();
                let existing_is_valid = existing_id
                    .as_ref()
                    .is_some_and(|id| devices.iter().any(|d| &d.device_id == id));
                if !existing_is_valid && let Some(first) = devices.first() {
                    mic.set_input_device_id(Some(first.device_id.clone()));
                }
            }
            devices_state.set(Some(result));
        });
    });

    let mic_change = mic.clone();
    let on_change = move |device_id: String| {
        mic_change.set_input_device_id(Some(device_id));
    };

    let mic_permission = mic.clone();
    let on_request_permission = move |_: Event<MouseData>| {
        requesting_permission.set(true);
        let mic = mic_permission.clone();
        spawn(async move {
            let result = request_microphone_permission().await;
            if let AudioInputDevicesResult::Available(ref devices) = result {
                let existing_id = mic.input_device_id();
                let existing_is_valid = existing_id
                    .as_ref()
                    .is_some_and(|id| devices.iter().any(|d| &d.device_id == id));
                if !existing_is_valid && let Some(first) = devices.first() {
                    mic.set_input_device_id(Some(first.device_id.clone()));
                }
            }
            devices_state.set(Some(result));
            requesting_permission.set(false);
        });
    };

    let on_retry = move |_: Event<MouseData>| {
        devices_state.set(None);
        let mic = mic.clone();
        spawn(async move {
            let result = enumerate_audio_input_devices().await;
            if let AudioInputDevicesResult::Available(ref devices) = result {
                let existing_id = mic.input_device_id();
                let existing_is_valid = existing_id
                    .as_ref()
                    .is_some_and(|id| devices.iter().any(|d| &d.device_id == id));
                if !existing_is_valid && let Some(first) = devices.first() {
                    mic.set_input_device_id(Some(first.device_id.clone()));
                }
            }
            devices_state.set(Some(result));
        });
    };

    rsx! {
        div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
            div {
                h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Настройки аудио" }
                p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Настройка устройств ввода, вывода и режима активации микрофона." }
            }

            div { class: "mt-4 grid gap-4 md:grid-cols-2",
                // Input device column.
                div { class: "space-y-4",
                    div { class: "block",
                        span { class: "mb-2 block text-[13px] font-medium text-zinc-300", "Устройство ввода" }
                        {input_device_widget(
                            devices_state(),
                            requesting_permission(),
                            selected_device_id,
                            on_change,
                            on_request_permission,
                            on_retry,
                        )}
                    }
                    {volume_slider("Громкость микрофона", input_volume(), move |value| input_volume.set(value))}
                }

                // Output device column (mock for now).
                div { class: "space-y-4",
                    label { class: "block",
                        span { class: "mb-2 block text-[13px] font-medium text-zinc-300", "Устройство вывода" }
                        select {
                            value: output_device(),
                            onchange: move |event| output_device.set(event.value()),
                            class: select_class(),
                            option { value: "SteelSeries Arctis 7", "SteelSeries Arctis 7" }
                            option { value: "Realtek Speakers", "Realtek Speakers" }
                            option { value: "HDMI Output", "HDMI Output" }
                        }
                    }
                    {volume_slider("Громкость вывода", output_volume(), move |value| output_volume.set(value))}
                }
            }

            div { class: "mt-4",
                div {
                    label { class: "mb-3 block text-[13px] font-medium text-zinc-300", "Режим активации" }
                    div { class: "grid gap-3 md:grid-cols-3",
                        button {
                            r#type: "button",
                            class: activation_button_class(activation_mode() == ActivationMode::AlwaysOn),
                            onclick: move |_| activation_mode.set(ActivationMode::AlwaysOn),
                            div { class: "font-medium", "Всегда включен" }
                            div { class: "mt-1 text-[12px] leading-4 text-zinc-400", "Микрофон активен постоянно." }
                        }

                        button {
                            r#type: "button",
                            class: activation_button_class(activation_mode() == ActivationMode::VoiceActivation),
                            onclick: move |_| activation_mode.set(ActivationMode::VoiceActivation),
                            div { class: "font-medium", "Активация по голосу" }
                            div { class: "mt-1 text-[12px] leading-4 text-zinc-400", "Включение при превышении порога." }
                        }

                        div { class: "group relative",
                            button {
                                r#type: "button",
                                disabled: true,
                                class: "w-full cursor-not-allowed rounded-2xl border border-zinc-800 bg-zinc-900/35 px-4 py-4 text-left opacity-60",
                                div { class: "font-medium", "Push-to-talk" }
                                div { class: "mt-1 text-[12px] leading-4 text-zinc-500", "Активация по удержанию клавиши." }
                            }
                            div { class: "pointer-events-none absolute left-1/2 top-full z-10 mt-2 w-max -translate-x-1/2 rounded-xl border border-zinc-800 bg-zinc-950/95 px-3 py-2 text-[12px] text-zinc-200 opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-opacity duration-200 group-hover:opacity-100",
                                "в разработке"
                            }
                        }
                    }
                }

                div {
                    "data-open": if activation_mode() == ActivationMode::VoiceActivation { "true" } else { "false" },
                    class: "mt-0 max-h-0 overflow-hidden opacity-0 transition-[max-height,opacity,transform,margin] duration-300 ease-out -translate-y-2 pointer-events-none data-[open=true]:mt-4 data-[open=true]:max-h-[280px] data-[open=true]:translate-y-0 data-[open=true]:opacity-100 data-[open=true]:pointer-events-auto",
                    div { class: "rounded-2xl border border-zinc-800 bg-zinc-900/45 p-4",
                        div { class: "mb-4 flex items-center justify-between gap-4",
                            div {
                                h4 { class: "text-[14px] font-semibold text-zinc-100", "Уровень активации" }
                                p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Настройте чувствительность голосовой активации." }
                            }
                            div { class: "text-right",
                                div { class: "text-[11px] text-zinc-500", "Порог" }
                                div { class: "text-[13px] font-medium text-zinc-200", "{activation_level()}%" }
                            }
                        }

                        div { class: "space-y-4",
                            div {
                                div { class: "mb-2 flex items-center justify-between",
                                    span { class: "text-[12px] text-zinc-300", "Текущая громкость" }
                                    span { class: "text-[12px] text-zinc-500", "{live_level}%" }
                                }
                                div { class: "relative h-3 overflow-hidden rounded-full bg-zinc-950/80 ring-1 ring-zinc-800",
                                    div {
                                        class: "h-full rounded-full bg-gradient-to-r from-emerald-500 via-yellow-400 to-red-500 transition-all duration-100",
                                        style: "width: {live_level}%;",
                                    }
                                    div {
                                        class: "absolute top-0 h-full w-1 bg-blue-100 shadow-[0_0_10px_rgba(147,197,253,0.75)]",
                                        style: "left: {activation_level()}%;",
                                    }
                                }
                            }

                            div {
                                div { class: "mb-2 flex items-center justify-between",
                                    label { class: "text-[12px] font-medium text-zinc-300", "Чувствительность активации" }
                                }
                                input {
                                    r#type: "range",
                                    min: "0",
                                    max: "100",
                                    value: activation_level(),
                                    oninput: move |event| activation_level.set(parse_percent(&event.value(), activation_level())),
                                    class: "w-full cursor-pointer accent-blue-500",
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn input_device_widget(
    state: Option<AudioInputDevicesResult>,
    requesting_permission: bool,
    selected: Option<String>,
    mut on_change: impl FnMut(String) + 'static,
    on_request_permission: impl FnMut(Event<MouseData>) + 'static,
    on_retry: impl FnMut(Event<MouseData>) + 'static,
) -> Element {
    match state {
        None => rsx! {
            div { class: "flex h-10 items-center gap-2 text-[13px] text-zinc-500",
                span { class: "inline-block h-3 w-3 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-400" }
                "Загрузка устройств…"
            }
        },

        Some(AudioInputDevicesResult::Available(devices)) => rsx! {
            select {
                value: selected.unwrap_or_default(),
                onchange: move |event| on_change(event.value()),
                class: select_class(),
                for device in devices {
                    option {
                        value: "{device.device_id}",
                        {device_display_label(&device)}
                    }
                }
            }
        },

        Some(AudioInputDevicesResult::PermissionRequired) => rsx! {
            div { class: "space-y-2",
                div { class: "flex items-start gap-2 rounded-xl border border-blue-500/20 bg-blue-500/8 px-3 py-2.5",
                    span { class: "mt-px shrink-0 text-blue-400", "🎙" }
                    p { class: "text-[12px] leading-5 text-blue-300",
                        "Для выбора устройства разрешите доступ к микрофону."
                    }
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
        },

        Some(AudioInputDevicesResult::PermissionDenied) => rsx! {
            div { class: "flex items-start gap-2 rounded-xl border border-red-500/20 bg-red-500/8 px-3 py-2.5",
                span { class: "mt-px shrink-0 text-red-400", "⊘" }
                p { class: "text-[12px] leading-5 text-red-300",
                    "Доступ к микрофону запрещён. Разрешите его в настройках браузера и обновите страницу."
                }
            }
        },

        Some(AudioInputDevicesResult::NoDevices) => rsx! {
            div { class: "space-y-2",
                div { class: "flex items-start gap-2 rounded-xl border border-amber-500/20 bg-amber-500/8 px-3 py-2.5",
                    span { class: "mt-px shrink-0 text-amber-400", "⚠" }
                    p { class: "text-[12px] leading-5 text-amber-300",
                        "Устройства ввода не обнаружены. Подключите микрофон и повторите."
                    }
                }
                button {
                    r#type: "button",
                    onclick: on_retry,
                    class: "flex h-9 w-full items-center justify-center rounded-xl border border-zinc-700 bg-zinc-900 px-4 text-[12px] font-medium text-zinc-300 transition hover:border-zinc-500 hover:text-zinc-100",
                    "Обновить список"
                }
            }
        },

        Some(AudioInputDevicesResult::NotSupported) => rsx! {
            div { class: "flex items-start gap-2 rounded-xl border border-zinc-700/50 bg-zinc-900/50 px-3 py-2.5",
                span { class: "mt-px shrink-0 text-zinc-500", "⊘" }
                p { class: "text-[12px] leading-5 text-zinc-500",
                    "Браузер не поддерживает выбор устройств аудиовхода."
                }
            }
        },
    }
}

fn device_display_label(device: &AudioInputDevice) -> String {
    if device.label.is_empty() {
        let preview: String = device.device_id.chars().take(8).collect();
        format!("Устройство ({preview}…)")
    } else {
        device.label.clone()
    }
}

fn volume_slider(
    label: &'static str,
    value: i32,
    mut on_change: impl FnMut(i32) + 'static,
) -> Element {
    rsx! {
        div {
            div { class: "mb-2 flex items-center justify-between",
                label { class: "text-[13px] font-medium text-zinc-300", "{label}" }
                span { class: "text-[12px] text-zinc-500", "{value}%" }
            }
            input {
                r#type: "range",
                min: "0",
                max: "100",
                value,
                oninput: move |event| on_change(parse_percent(&event.value(), value)),
                class: "w-full cursor-pointer accent-blue-500",
            }
        }
    }
}

fn activation_button_class(active: bool) -> &'static str {
    if active {
        "relative rounded-2xl border border-accent/30 bg-accent/10 px-4 py-4 text-left transition hover:border-blue-400/45"
    } else {
        "relative rounded-2xl border border-zinc-700 bg-zinc-950 px-4 py-4 text-left transition hover:border-zinc-500"
    }
}

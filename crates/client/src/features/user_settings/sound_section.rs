//! User sound settings section.

use dioxus::prelude::*;

use crate::features::audio_playback::{
    AudioOutputDevice, AudioOutputDevicesResult, AudioPlaybackHandle,
    enumerate_audio_output_devices,
};
use crate::features::microphone::{
    AudioInputDevice, AudioInputDevicesResult, MicrophoneHandle, enumerate_audio_input_devices,
    request_microphone_permission,
};

use super::sound_devices::{input_device_widget, output_device_widget};
use super::styles::parse_percent;

#[derive(Clone, Copy, PartialEq)]
enum ActivationMode {
    AlwaysOn,
    VoiceActivation,
}

/// Renders sound input, output, and voice activation controls.
#[component]
pub(crate) fn SoundSettingsSection() -> Element {
    let mic = use_context::<MicrophoneHandle>();
    let playback = use_context::<AudioPlaybackHandle>();

    // Read the stored device preference — this creates a reactive subscription so the
    // select re-renders whenever the stored value changes.
    let selected_input_device_id = mic.input_device_id();
    let selected_output_device_id = playback.output_device_id();

    let mut input_devices_state = use_signal(|| Option::<AudioInputDevicesResult>::None);
    let mut output_devices_state = use_signal(|| Option::<AudioOutputDevicesResult>::None);
    let requesting_permission = use_signal(|| false);

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
                if mic.input_device_id().is_none() {
                    select_first_input_device(&mic, devices);
                } else {
                    mic.reconcile_input_devices(devices);
                }
            }
            input_devices_state.set(Some(result));
        });
    });

    let playback_effect = playback.clone();
    use_effect(move || {
        let playback = playback_effect.clone();
        spawn(async move {
            let result = enumerate_audio_output_devices().await;
            if let AudioOutputDevicesResult::Available(ref devices) = result {
                if playback.output_device_id().is_none() {
                    select_first_output_device(&playback, devices);
                } else {
                    playback.reconcile_output_devices(devices);
                }
            }
            output_devices_state.set(Some(result));
        });
    });

    let mic_change = mic.clone();
    let on_input_change = move |device: AudioInputDevice| {
        mic_change.set_input_device(&device);
    };

    let playback_change = playback.clone();
    let on_output_change = move |device: AudioOutputDevice| {
        playback_change.set_output_device(&device);
    };

    let mic_input_permission = mic.clone();
    let playback_input_permission = playback.clone();
    let on_input_request_permission = move |_: Event<MouseData>| {
        refresh_devices_after_permission(
            mic_input_permission.clone(),
            playback_input_permission.clone(),
            input_devices_state,
            output_devices_state,
            requesting_permission,
        );
    };

    let mic_output_permission = mic.clone();
    let playback_output_permission = playback.clone();
    let on_output_request_permission = move |_: Event<MouseData>| {
        refresh_devices_after_permission(
            mic_output_permission.clone(),
            playback_output_permission.clone(),
            input_devices_state,
            output_devices_state,
            requesting_permission,
        );
    };

    let retry_input_mic = mic.clone();
    let retry_input_playback = playback.clone();
    let on_input_retry = move |_: Event<MouseData>| {
        refresh_devices(
            retry_input_mic.clone(),
            retry_input_playback.clone(),
            input_devices_state,
            output_devices_state,
        );
    };

    let retry_output_mic = mic.clone();
    let retry_output_playback = playback.clone();
    let on_output_retry = move |_: Event<MouseData>| {
        refresh_devices(
            retry_output_mic.clone(),
            retry_output_playback.clone(),
            input_devices_state,
            output_devices_state,
        );
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
                            input_devices_state(),
                            requesting_permission(),
                            selected_input_device_id,
                            on_input_change,
                            on_input_request_permission,
                            on_input_retry,
                        )}
                    }
                    {volume_slider("Громкость микрофона", input_volume(), move |value| input_volume.set(value))}
                }

                // Output device column (mock for now).
                div { class: "space-y-4",
                    div { class: "block",
                        span { class: "mb-2 block text-[13px] font-medium text-zinc-300", "Устройство вывода" }
                        {output_device_widget(
                            output_devices_state(),
                            requesting_permission(),
                            selected_output_device_id,
                            on_output_change,
                            on_output_request_permission,
                            on_output_retry,
                        )}
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

fn select_first_input_device(mic: &MicrophoneHandle, devices: &[AudioInputDevice]) {
    if let Some(first) = devices.first() {
        mic.set_input_device(first);
    }
}

fn select_first_output_device(playback: &AudioPlaybackHandle, devices: &[AudioOutputDevice]) {
    if let Some(first) = devices.first() {
        playback.set_output_device(first);
    }
}

fn refresh_devices_after_permission(
    mic: MicrophoneHandle,
    playback: AudioPlaybackHandle,
    input_devices_state: Signal<Option<AudioInputDevicesResult>>,
    output_devices_state: Signal<Option<AudioOutputDevicesResult>>,
    mut requesting_permission: Signal<bool>,
) {
    requesting_permission.set(true);
    spawn(async move {
        refresh_devices_inner(
            mic,
            playback,
            input_devices_state,
            output_devices_state,
            true,
        )
        .await;
        requesting_permission.set(false);
    });
}

fn refresh_devices(
    mic: MicrophoneHandle,
    playback: AudioPlaybackHandle,
    mut input_devices_state: Signal<Option<AudioInputDevicesResult>>,
    mut output_devices_state: Signal<Option<AudioOutputDevicesResult>>,
) {
    input_devices_state.set(None);
    output_devices_state.set(None);
    spawn(async move {
        refresh_devices_inner(
            mic,
            playback,
            input_devices_state,
            output_devices_state,
            false,
        )
        .await;
    });
}

async fn refresh_devices_inner(
    mic: MicrophoneHandle,
    playback: AudioPlaybackHandle,
    mut input_devices_state: Signal<Option<AudioInputDevicesResult>>,
    mut output_devices_state: Signal<Option<AudioOutputDevicesResult>>,
    request_permission: bool,
) {
    let input_result = if request_permission {
        request_microphone_permission().await
    } else {
        enumerate_audio_input_devices().await
    };
    if let AudioInputDevicesResult::Available(ref devices) = input_result {
        if mic.input_device_id().is_none() {
            select_first_input_device(&mic, devices);
        } else {
            mic.reconcile_input_devices(devices);
        }
    }
    input_devices_state.set(Some(input_result));

    let output_result = enumerate_audio_output_devices().await;
    if let AudioOutputDevicesResult::Available(ref devices) = output_result {
        if playback.output_device_id().is_none() {
            select_first_output_device(&playback, devices);
        } else {
            playback.reconcile_output_devices(devices);
        }
    }
    output_devices_state.set(Some(output_result));
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

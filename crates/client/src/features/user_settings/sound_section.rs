//! User sound settings section.

use dioxus::prelude::*;

use super::styles::{parse_percent, select_class};

#[derive(Clone, Copy, PartialEq)]
enum ActivationMode {
    AlwaysOn,
    VoiceActivation,
}

/// Renders mock sound input, output, and voice activation controls.
#[component]
pub(crate) fn SoundSettingsSection() -> Element {
    let mut input_device = use_signal(|| "HyperX QuadCast".to_owned());
    let mut output_device = use_signal(|| "SteelSeries Arctis 7".to_owned());
    let mut input_volume = use_signal(|| 75);
    let mut output_volume = use_signal(|| 60);
    let mut activation_mode = use_signal(|| ActivationMode::AlwaysOn);
    let mut activation_level = use_signal(|| 45);
    let live_level = 58;

    rsx! {
        div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
            div {
                h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Настройки аудио" }
                p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Настройка устройств ввода, вывода и режима активации микрофона." }
            }

            div { class: "mt-4 grid gap-4 md:grid-cols-2",
                div { class: "space-y-4",
                    label { class: "block",
                        span { class: "mb-2 block text-[13px] font-medium text-zinc-300", "Устройство ввода" }
                        select {
                            value: input_device(),
                            onchange: move |event| input_device.set(event.value()),
                            class: select_class(),
                            option { value: "HyperX QuadCast", "HyperX QuadCast" }
                            option { value: "Blue Yeti", "Blue Yeti" }
                            option { value: "USB Audio Device", "USB Audio Device" }
                        }
                    }
                    {volume_slider("Громкость микрофона", input_volume(), move |value| input_volume.set(value))}
                }

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

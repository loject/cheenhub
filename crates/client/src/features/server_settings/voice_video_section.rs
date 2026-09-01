//! Секция настроек голоса и видео сервера.

use cheenhub_contracts::media::{VOICE_AUDIO_BITRATE_MAX_BPS, VOICE_AUDIO_BITRATE_MIN_BPS};
use cheenhub_contracts::rest::ServerVoiceSettings;
use cheenhub_contracts::video_presets::VideoPresetId;
use dioxus::prelude::*;

use super::api;

const AUDIO_CODEC: &str = "Opus";
const VIDEO_CODEC: &str = "VP9";
const AUDIO_BITRATE_MIN_KBPS: u32 = VOICE_AUDIO_BITRATE_MIN_BPS / 1_000;
const AUDIO_BITRATE_MAX_KBPS: u32 = VOICE_AUDIO_BITRATE_MAX_BPS / 1_000;
const AUDIO_BITRATE_STEP_KBPS: u32 = 8;

struct VideoProfile {
    title: &'static str,
    description: &'static str,
    bitrate_label: &'static str,
    bitrate_kbps: u32,
    bitrate_max_kbps: u32,
    resolution: &'static str,
    fps: &'static str,
}

const CAMERA_PROFILE: VideoProfile = VideoProfile {
    title: "Камера",
    description: "Пределы видео для камеры на этом сервере.",
    bitrate_label: "700 кбит/с",
    bitrate_kbps: VideoPresetId::Camera720p24.spec().bitrate_bps / 1_000,
    bitrate_max_kbps: 2_000,
    resolution: "1280x720",
    fps: "24 fps",
};

const SCREEN_SHARE_PROFILE: VideoProfile = VideoProfile {
    title: "Демонстрация экрана",
    description: "Пределы видео для показа экрана на этом сервере.",
    bitrate_label: "1.5 Мбит/с",
    bitrate_kbps: VideoPresetId::Screen720p30.spec().bitrate_bps / 1_000,
    bitrate_max_kbps: 2_000,
    resolution: "1280x720 или 1920x1080",
    fps: "30 fps или 15 fps",
};

fn clamp_bitrate_kbps(value: u32) -> u32 {
    value.clamp(AUDIO_BITRATE_MIN_KBPS, AUDIO_BITRATE_MAX_KBPS)
}

/// Рендерит секцию настроек качества голоса и видео сервера.
#[component]
pub(crate) fn ServerVoiceVideoSettingsSection(server_id: String, is_owner: bool) -> Element {
    let mount_server_id = server_id.clone();
    use_hook(move || {
        info!(
            server_id = %mount_server_id,
            settings_section = "voice_video",
            "opened server voice and video settings section"
        );
    });

    let load_server_id = server_id.clone();
    let mut settings_load = use_resource(move || {
        let request_server_id = load_server_id.clone();
        async move { api::get_voice_settings(request_server_id).await }
    });
    let mut saved_bitrate_bps = use_signal(|| None::<u32>);
    let mut draft_kbps = use_signal(|| None::<u32>);
    let mut load_error = use_signal(String::new);
    let save_error = use_signal(String::new);
    let is_saving = use_signal(|| false);

    let effect_server_id = server_id.clone();
    use_effect(move || {
        if saved_bitrate_bps().is_some() {
            return;
        }
        let Some(result) = settings_load.read().clone() else {
            return;
        };
        match result {
            Ok(settings) => {
                info!(
                    server_id = %effect_server_id,
                    audio_bitrate_bps = settings.audio_bitrate_bps,
                    "loaded server voice settings in settings ui"
                );
                saved_bitrate_bps.set(Some(settings.audio_bitrate_bps));
                draft_kbps.set(Some(clamp_bitrate_kbps(settings.audio_bitrate_bps / 1_000)));
                load_error.set(String::new());
            }
            Err(error) => {
                warn!(%error, server_id = %effect_server_id, "failed to load server voice settings in settings ui");
                load_error.set(error);
            }
        }
    });

    let is_loading = saved_bitrate_bps().is_none() && settings_load.read().is_none();
    let loaded_kbps = saved_bitrate_bps().map(|bitrate| clamp_bitrate_kbps(bitrate / 1_000));

    rsx! {
        div { class: "space-y-4 pb-24 xl:pb-0",
            div { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
                div { class: "flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between",
                    div { class: "min-w-0",
                        h3 { class: "text-[18px] font-semibold text-zinc-50", "Голос и видео" }
                        p { class: "mt-2 max-w-2xl text-[13px] leading-6 text-zinc-500",
                            "Параметры качества, которые применяются к голосовым комнатам сервера."
                        }
                    }
                }
            }

            div { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
                {section_heading(
                    "Аудио",
                    "Параметры голоса и целевой битрейт микрофона.",
                )}
                if is_loading {
                    div { class: "mt-5 space-y-3",
                        div { class: "h-10 animate-pulse rounded-xl border border-zinc-800 bg-zinc-900/55" }
                        div { class: "h-10 animate-pulse rounded-xl border border-zinc-800 bg-zinc-900/40" }
                    }
                } else if !load_error().is_empty() && saved_bitrate_bps().is_none() {
                    div { class: "mt-5 rounded-2xl border border-red-500/20 bg-red-500/10 p-4",
                        p { class: "text-[13px] font-medium text-red-100", "Не удалось загрузить настройки голоса" }
                        p { class: "mt-1 text-[12px] leading-5 text-red-200", "{load_error()}" }
                        button {
                            r#type: "button",
                            class: "mt-3 rounded-xl border border-zinc-700 bg-zinc-900/80 px-4 py-2 text-[12px] font-medium text-zinc-200 transition hover:border-zinc-600 hover:bg-zinc-900",
                            onclick: move |_| {
                                load_error.set(String::new());
                                settings_load.restart();
                            },
                            "Повторить"
                        }
                    }
                } else {
                    div { class: "mt-5 grid gap-4 lg:grid-cols-2",
                        {single_option_select("Кодек", AUDIO_CODEC)}
                        if is_owner && loaded_kbps.is_some() {
                            {editable_bitrate_range(
                                server_id.clone(),
                                saved_bitrate_bps,
                                draft_kbps,
                                save_error,
                                is_saving,
                            )}
                        } else if let Some(kbps) = loaded_kbps {
                            {locked_range(
                                "Битрейт",
                                &format!("{kbps} кбит/с"),
                                kbps,
                                AUDIO_BITRATE_MAX_KBPS,
                            )}
                        }
                    }
                }
            }

            div { class: "grid gap-4 xl:grid-cols-2",
                {video_profile_card(CAMERA_PROFILE)}
                {video_profile_card(SCREEN_SHARE_PROFILE)}
            }
        }
    }
}

fn editable_bitrate_range(
    server_id: String,
    mut saved_bitrate_bps: Signal<Option<u32>>,
    mut draft_kbps: Signal<Option<u32>>,
    mut save_error_signal: Signal<String>,
    mut is_saving: Signal<bool>,
) -> Element {
    let draft_value = draft_kbps().unwrap_or(AUDIO_BITRATE_MIN_KBPS);
    let is_dirty = match (draft_kbps(), saved_bitrate_bps()) {
        (Some(kbps), Some(saved_bps)) => kbps * 1_000 != saved_bps,
        _ => false,
    };
    let saving = is_saving();
    let save_error = save_error_signal();

    rsx! {
        div { class: "group/bitrate relative block min-w-0", tabindex: "0",
            div { class: "mb-2 flex items-center justify-between gap-3",
                label { class: "text-[13px] font-medium text-zinc-300", "Битрейт" }
                span { class: "shrink-0 text-[12px] font-medium text-zinc-200", "{draft_value} кбит/с" }
            }
            input {
                r#type: "range",
                min: "{AUDIO_BITRATE_MIN_KBPS}",
                max: "{AUDIO_BITRATE_MAX_KBPS}",
                step: "{AUDIO_BITRATE_STEP_KBPS}",
                value: "{draft_value}",
                oninput: move |event| {
                    let parsed: u32 = event.value().parse().unwrap_or(AUDIO_BITRATE_MIN_KBPS);
                    draft_kbps.set(Some(clamp_bitrate_kbps(parsed)));
                },
                class: "h-10 w-full cursor-pointer accent-blue-500",
                "aria-label": "Битрейт аудио, кбит/с",
            }
            div { class: "mt-3 flex min-h-8 items-center gap-3",
                button {
                    r#type: "button",
                    disabled: saving || !is_dirty,
                    class: if saving || !is_dirty {
                        "rounded-xl border border-zinc-800 bg-zinc-900/60 px-4 py-2 text-[12px] font-medium text-zinc-500 cursor-not-allowed"
                    } else {
                        "rounded-xl border border-accent/40 bg-accent px-4 py-2 text-[12px] font-medium text-white transition hover:bg-blue-400"
                    },
                    onclick: move |event| {
                        event.prevent_default();
                        if is_saving() || !is_dirty {
                            return;
                        }
                        let Some(kbps) = draft_kbps() else {
                            return;
                        };
                        let save_server_id = server_id.clone();
                        is_saving.set(true);
                        save_error_signal.set(String::new());
                        spawn(async move {
                            let result = api::update_voice_settings(
                                save_server_id.clone(),
                                ServerVoiceSettings {
                                    audio_bitrate_bps: kbps * 1_000,
                                },
                            )
                            .await;
                            is_saving.set(false);
                            match result {
                                Ok(settings) => {
                                    info!(
                                        server_id = %save_server_id,
                                        audio_bitrate_bps = settings.audio_bitrate_bps,
                                        "saved server voice settings in settings ui"
                                    );
                                    saved_bitrate_bps.set(Some(settings.audio_bitrate_bps));
                                    save_error_signal.set(String::new());
                                }
                                Err(error) => {
                                    warn!(%error, server_id = %save_server_id, "failed to save server voice settings in settings ui");
                                    save_error_signal.set(error);
                                }
                            }
                        });
                    },
                    if saving { "Сохраняем..." } else { "Сохранить" }
                }
                if !save_error.is_empty() {
                    span { class: "text-[12px] leading-5 text-red-300", "{save_error}" }
                }
            }
        }
    }
}

fn section_heading(title: &'static str, description: &'static str) -> Element {
    rsx! {
        div {
            h4 { class: "text-[16px] font-semibold text-zinc-50", "{title}" }
            p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "{description}" }
        }
    }
}

fn video_profile_card(profile: VideoProfile) -> Element {
    rsx! {
        div { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
            {section_heading(profile.title, profile.description)}
            div { class: "mt-5 grid gap-4 sm:grid-cols-2",
                {single_option_select("Кодек", VIDEO_CODEC)}
                {locked_range(
                    "Битрейт",
                    profile.bitrate_label,
                    profile.bitrate_kbps,
                    profile.bitrate_max_kbps,
                )}
                {single_option_select("Максимальное разрешение видео", profile.resolution)}
                {single_option_select("Частота кадров", profile.fps)}
            }
        }
    }
}

fn single_option_select(label: &'static str, value: &'static str) -> Element {
    rsx! {
        label { class: "block min-w-0",
            span { class: "mb-2 block text-[13px] font-medium text-zinc-300", "{label}" }
            select {
                class: single_option_control_class(),
                value,
                option { value, "{value}" }
            }
        }
    }
}

fn locked_range(label: &str, value_label: &str, value: u32, max: u32) -> Element {
    rsx! {
        div { class: "group/bitrate relative block min-w-0", tabindex: "0",
            div { class: "mb-2 flex items-center justify-between gap-3",
                label { class: "text-[13px] font-medium text-zinc-300", "{label}" }
                span { class: "shrink-0 text-[12px] font-medium text-zinc-200", "{value_label}" }
            }
            input {
                r#type: "range",
                min: "0",
                max: "{max}",
                value: "{value}",
                disabled: true,
                class: "h-10 w-full cursor-default accent-blue-500 disabled:opacity-80",
                "aria-label": "{label}: {value_label}",
            }
        }
    }
}

fn single_option_control_class() -> &'static str {
    "h-10 w-full cursor-pointer rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[13px] text-zinc-100 outline-none transition focus:border-accent/70 focus:ring-4 focus:ring-accent/10"
}

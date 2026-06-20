//! Секция настроек голоса и видео сервера.

use dioxus::prelude::*;

const AUDIO_CODEC: &str = "Opus";
const AUDIO_BITRATE_LABEL: &str = "32 кбит/с";
const AUDIO_BITRATE_KBPS: u32 = 32;
const AUDIO_BITRATE_MAX_KBPS: u32 = 64;

const VIDEO_CODEC: &str = "VP9";

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
    bitrate_kbps: 700,
    bitrate_max_kbps: 2_000,
    resolution: "640x360",
    fps: "24 fps",
};

const SCREEN_SHARE_PROFILE: VideoProfile = VideoProfile {
    title: "Демонстрация экрана",
    description: "Пределы видео для показа экрана на этом сервере.",
    bitrate_label: "1.5 Мбит/с",
    bitrate_kbps: 1_500,
    bitrate_max_kbps: 2_000,
    resolution: "1280x720",
    fps: "30 fps",
};

/// Рендерит секцию настроек качества голоса и видео сервера.
#[component]
pub(crate) fn ServerVoiceVideoSettingsSection(server_id: String) -> Element {
    let mount_server_id = server_id.clone();
    use_hook(move || {
        info!(
            server_id = %mount_server_id,
            settings_section = "voice_video",
            "opened server voice and video settings section"
        );
    });

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
                div { class: "mt-5 grid gap-4 lg:grid-cols-2",
                    {single_option_select("Кодек", AUDIO_CODEC)}
                    {locked_range(
                        "Битрейт",
                        AUDIO_BITRATE_LABEL,
                        AUDIO_BITRATE_KBPS,
                        AUDIO_BITRATE_MAX_KBPS,
                    )}
                }
            }

            div { class: "grid gap-4 xl:grid-cols-2",
                {video_profile_card(CAMERA_PROFILE)}
                {video_profile_card(SCREEN_SHARE_PROFILE)}
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

fn locked_range(label: &'static str, value_label: &'static str, value: u32, max: u32) -> Element {
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
            span {
                role: "tooltip",
                class: "pointer-events-none absolute right-0 top-[calc(100%+8px)] z-20 w-max max-w-[220px] translate-y-1 rounded-xl border border-zinc-800 bg-zinc-950/95 px-3 py-2 text-[12px] leading-4 text-zinc-200 opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-[opacity,transform] duration-150 group-hover/bitrate:translate-y-0 group-hover/bitrate:opacity-100 group-focus/bitrate:translate-y-0 group-focus/bitrate:opacity-100",
                "Настройка битрейта в разработке."
            }
        }
    }
}

fn single_option_control_class() -> &'static str {
    "h-10 w-full cursor-pointer rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[13px] text-zinc-100 outline-none transition focus:border-accent/70 focus:ring-4 focus:ring-accent/10"
}

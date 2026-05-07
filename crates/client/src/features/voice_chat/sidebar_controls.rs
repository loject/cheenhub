//! Sidebar voice connection controls.

use dioxus::prelude::*;

use super::state::{VoiceConnectionHandle, VoiceConnectionState};

/// Renders animated sidebar voice controls for the active voice connection.
#[component]
pub(crate) fn SidebarVoiceControls() -> Element {
    let voice = use_context::<VoiceConnectionHandle>();
    let state = voice.state();
    let visible = state.shows_sidebar_controls();
    let target = state.active_target();
    let room_name = target
        .as_ref()
        .map(|target| target.room_name.clone())
        .unwrap_or_else(|| "Нет активной комнаты".to_owned());
    let status = match &state {
        VoiceConnectionState::Disconnected => "не подключен",
        VoiceConnectionState::Connecting { .. } => "подключаемся",
        VoiceConnectionState::Connected { participants, .. } => {
            if participants.len() == 1 {
                "1 участник"
            } else {
                "в голосовой комнате"
            }
        }
        VoiceConnectionState::Disconnecting { .. } => "отключаемся",
        VoiceConnectionState::Error { .. } => "нужна повторная попытка",
    };
    let error = match &state {
        VoiceConnectionState::Error { message, .. } => Some(message.clone()),
        _ => None,
    };

    rsx! {
        div { class: "sidebar-voice-presence", "data-sidebar-voice-open": if visible { "true" } else { "false" },
            div { class: "sidebar-voice-presence-inner relative mb-2 rounded-[20px] border border-zinc-800 bg-zinc-900/80 p-2.5",
                div { class: "mb-2 flex items-start gap-2",
                    div { class: "min-w-0 flex-1",
                        div { class: "truncate text-[11px] font-medium text-zinc-100", "{room_name}" }
                        div { class: "mt-0.5 flex min-w-0 items-center gap-1.5 text-[11px] text-zinc-500",
                            svg { class: "h-3.5 w-3.5 shrink-0 text-zinc-600", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11a7 7 0 0 1-14 0m7 8v3m-4 0h8m-4-18a3 3 0 0 0-3 3v4a3 3 0 1 0 6 0V7a3 3 0 0 0-3-3Z" }
                            }
                            span { class: "truncate", "{status}" }
                        }
                    }
                }
                div { class: "grid grid-cols-4 gap-2",
                    button { r#type: "button", class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/70 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900", "aria-label": "Выключить микрофон",
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11a7 7 0 0 1-14 0m7 8v3m-4 0h8m-4-18a3 3 0 0 0-3 3v4a3 3 0 1 0 6 0V7a3 3 0 0 0-3-3Z" }
                        }
                    }
                    button { r#type: "button", class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/70 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900", "aria-label": "Отключить звук",
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19.114 5.636a9 9 0 0 1 0 12.728M16.463 8.288a5.25 5.25 0 0 1 0 7.424M6.75 8.25 10.5 4.5v15l-3.75-3.75H3.75A1.5 1.5 0 0 1 2.25 14.25v-4.5A1.5 1.5 0 0 1 3.75 8.25h3Z" }
                        }
                    }
                    button { r#type: "button", class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/70 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900", "aria-label": "Параметры голосового чата",
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10.5 6h9.75M10.5 6a1.5 1.5 0 1 1-3 0m3 0a1.5 1.5 0 1 0-3 0M3.75 6H7.5m9 6h3.75m-3.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-9.75 0h9.75m-3 6h9.75m-9.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-3.75 0H7.5" }
                        }
                    }
                    button {
                        r#type: "button",
                        disabled: matches!(state, VoiceConnectionState::Disconnecting { .. }),
                        class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-red-500/35 hover:bg-red-500/10 hover:text-red-200 disabled:cursor-wait disabled:opacity-60",
                        "aria-label": "Выйти из голосового чата",
                        onclick: move |_| voice.leave(),
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9" }
                        }
                    }
                }
                if let Some(error) = error {
                    p { class: "mt-2 rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[11px] leading-4 text-red-200", "{error}" }
                }
            }
        }
    }
}

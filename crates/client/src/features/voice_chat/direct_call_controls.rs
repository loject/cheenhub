//! Панель основных действий активного личного звонка.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::features::app::current_user::CurrentUserContext;
use crate::features::camera::{CameraHandle, CameraStatus};
use crate::features::microphone::{MicrophoneHandle, MicrophoneStatus};
use crate::features::realtime::RealtimeHandle;
use crate::features::screen_share::{ScreenShareHandle, ScreenShareStatus};

use super::android_output_route_button::{
    AndroidOutputRouteButton, output_route_control_available,
};
use super::direct_call_controls_visibility::DirectCallControlsVisibility;
use super::direct_call_state::DirectCallHandle;
use super::state::{VoiceConnectionHandle, VoiceConnectionState, VoiceRoomTarget};
use super::video_streams::{ParticipantVideoFrame, ParticipantVideoHandle, ParticipantVideoSource};
use super::{microphone_uplink, realtime};

/// Показывает стабильный dock личного звонка и выполняет его команды.
#[component]
pub(crate) fn DirectCallControls(
    target: VoiceRoomTarget,
    chat_open: bool,
    on_toggle_chat: EventHandler<()>,
) -> Element {
    let voice = use_context::<VoiceConnectionHandle>();
    let direct_call = use_context::<DirectCallHandle>();
    let microphone = use_context::<MicrophoneHandle>();
    let camera = use_context::<CameraHandle>();
    let screen_share = use_context::<ScreenShareHandle>();
    let realtime_handle = use_context::<RealtimeHandle>();
    let current_user_id = use_context::<CurrentUserContext>().require_user().id;
    let participant_video = use_context::<ParticipantVideoHandle>();
    let controls_visibility = use_context::<DirectCallControlsVisibility>();
    let output_route_available = output_route_control_available();
    let mut more_open = use_signal(|| false);
    let state = voice.state();
    let microphone_status = microphone.status();
    let camera_status = camera.status();
    let screen_share_status = screen_share.status();
    let microphone_level = microphone.level();
    let is_active_call = state
        .active_target()
        .is_some_and(|active| active.matches(&target));
    let media_controls_enabled = matches!(state, VoiceConnectionState::Connected { .. });
    let is_leaving =
        matches!(state, VoiceConnectionState::Disconnecting { .. }) || direct_call.busy();
    let microphone_live = matches!(microphone_status, MicrophoneStatus::Live);
    let microphone_starting = matches!(microphone_status, MicrophoneStatus::Starting);
    let microphone_speaking = microphone_live && microphone_level.active;
    let camera_live = matches!(camera_status, CameraStatus::Live);
    let camera_starting = matches!(camera_status, CameraStatus::Starting);
    let screen_share_live = matches!(screen_share_status, ScreenShareStatus::Live);
    let screen_share_starting = matches!(screen_share_status, ScreenShareStatus::Starting);
    let microphone_label = match &microphone_status {
        MicrophoneStatus::Idle => "Включить микрофон".to_owned(),
        MicrophoneStatus::Starting => "Запрашиваем микрофон".to_owned(),
        MicrophoneStatus::Live => "Выключить микрофон".to_owned(),
        MicrophoneStatus::PermissionDenied => "Доступ к микрофону запрещён".to_owned(),
        MicrophoneStatus::Error(message) => message.clone(),
    };
    let camera_label = match camera_status {
        CameraStatus::Idle => "Включить камеру",
        CameraStatus::Starting => "Запрашиваем камеру",
        CameraStatus::Live => "Выключить камеру",
        CameraStatus::PermissionDenied => "Доступ к камере запрещён",
        CameraStatus::Error(_) => "Камера недоступна",
    };
    let screen_share_label = match screen_share_status {
        ScreenShareStatus::Idle => "Начать демонстрацию экрана",
        ScreenShareStatus::Starting => "Запрашиваем демонстрацию экрана",
        ScreenShareStatus::Live => "Остановить демонстрацию экрана",
        ScreenShareStatus::PermissionDenied => "Доступ к экрану запрещён",
        ScreenShareStatus::Error(_) => "Демонстрация экрана недоступна",
    };

    if !is_active_call {
        return rsx! {};
    }

    let microphone_button_class = if microphone_speaking {
        "voice-control-button direct-call-control group relative flex h-12 w-12 items-center justify-center rounded-xl bg-blue-500/16 text-blue-100 shadow-[0_0_0_1px_rgba(96,165,250,.40)] transition-[scale,background-color,box-shadow,opacity] duration-[180ms] active:scale-[0.96] disabled:cursor-wait disabled:opacity-60"
    } else if microphone_live {
        "voice-control-button direct-call-control group relative flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-950/65 text-zinc-100 shadow-[0_0_0_1px_rgba(255,255,255,.08)] transition-[scale,background-color,box-shadow,opacity] duration-[180ms] active:scale-[0.96] disabled:cursor-wait disabled:opacity-60"
    } else {
        "voice-control-button direct-call-control group relative flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-950/65 text-zinc-300 shadow-[0_0_0_1px_rgba(255,255,255,.08)] transition-[scale,background-color,box-shadow,opacity] duration-[180ms] active:scale-[0.96] disabled:cursor-wait disabled:opacity-60"
    };
    let camera_button_class = if camera_live {
        "voice-control-button direct-call-control group relative flex h-12 w-12 items-center justify-center rounded-xl bg-blue-500/16 text-blue-100 shadow-[0_0_0_1px_rgba(96,165,250,.30)] transition-[scale,background-color,box-shadow,opacity] duration-[180ms] active:scale-[0.96] disabled:cursor-wait disabled:opacity-60"
    } else if matches!(
        camera_status,
        CameraStatus::PermissionDenied | CameraStatus::Error(_)
    ) {
        "voice-control-button direct-call-control group relative flex h-12 w-12 items-center justify-center rounded-xl bg-amber-500/12 text-amber-100 shadow-[0_0_0_1px_rgba(245,158,11,.25)] transition-[scale,background-color,box-shadow,opacity] duration-[180ms] active:scale-[0.96] disabled:cursor-wait disabled:opacity-60"
    } else {
        "voice-control-button direct-call-control group relative flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-950/65 text-zinc-200 shadow-[0_0_0_1px_rgba(255,255,255,.08)] transition-[scale,background-color,box-shadow,opacity] duration-[180ms] active:scale-[0.96] disabled:cursor-wait disabled:opacity-60"
    };
    let more_button_class = if more_open() || screen_share_live || chat_open {
        "voice-control-button direct-call-control relative flex h-12 w-12 items-center justify-center rounded-xl bg-blue-500/16 text-blue-100 shadow-[0_0_0_1px_rgba(96,165,250,.30)] transition-[scale,background-color,box-shadow] duration-[180ms] active:scale-[0.96]"
    } else {
        "voice-control-button direct-call-control relative flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-950/65 text-zinc-200 shadow-[0_0_0_1px_rgba(255,255,255,.08)] transition-[scale,background-color,box-shadow] duration-[180ms] active:scale-[0.96]"
    };
    let toggle_microphone = microphone.clone();
    let microphone_realtime = realtime_handle.clone();
    let microphone_server_id = target.server_id.clone();
    let microphone_room_id = target.room_id.clone();
    let toggle_camera = camera.clone();
    let camera_realtime = realtime_handle.clone();
    let camera_server_id = target.server_id.clone();
    let camera_room_id = target.room_id.clone();
    let camera_user_id = current_user_id.clone();
    let camera_video = participant_video.clone();
    let toggle_screen_share_handle = screen_share.clone();
    let screen_share_realtime = realtime_handle.clone();
    let screen_share_server_id = target.server_id.clone();
    let screen_share_room_id = target.room_id.clone();
    let leave_microphone = microphone.clone();
    let leave_camera = camera.clone();
    let leave_screen_share = screen_share.clone();
    let leave_direct_call = direct_call.clone();
    let leave_target = target.clone();

    let controls_class = if controls_visibility.is_visible() {
        "voice-floating-controls direct-call-controls pointer-events-none absolute inset-x-0 bottom-0 z-50 flex translate-y-0 justify-center p-5 opacity-100 transition-[opacity,transform] duration-200 ease-out"
    } else {
        "voice-floating-controls direct-call-controls pointer-events-none absolute inset-x-0 bottom-0 z-50 flex translate-y-3 justify-center p-5 opacity-0 transition-[opacity,transform] duration-200 ease-in"
    };

    rsx! {
        div {
            class: controls_class,
            "data-output-route": if output_route_available { "true" } else { "false" },
            div { class: if controls_visibility.is_visible() { "voice-controls-panel pointer-events-auto relative z-20 flex items-center gap-1.5 rounded-[18px] bg-zinc-900/88 p-1.5 shadow-[0_0_0_1px_rgba(255,255,255,.08),0_14px_38px_rgba(0,0,0,.34)] backdrop-blur-xl" } else { "voice-controls-panel pointer-events-none relative z-20 flex items-center gap-1.5 rounded-[18px] bg-zinc-900/88 p-1.5 shadow-[0_0_0_1px_rgba(255,255,255,.08),0_14px_38px_rgba(0,0,0,.34)] backdrop-blur-xl" },
                button {
                    r#type: "button",
                    disabled: !media_controls_enabled || microphone_starting,
                    class: microphone_button_class,
                    "aria-label": microphone_label,
                    onclick: move |_| {
                        if media_controls_enabled {
                            microphone_uplink::toggle(
                                toggle_microphone.clone(),
                                microphone_realtime.clone(),
                                microphone_server_id.clone(),
                                microphone_room_id.clone(),
                            );
                        }
                    },
                    span { class: control_icon_class(microphone_live), {microphone_icon(false)} }
                    span { class: control_icon_class(!microphone_live), {microphone_icon(true)} }
                }
                button {
                    r#type: "button",
                    disabled: !media_controls_enabled || camera_starting,
                    class: camera_button_class,
                    "aria-label": camera_label,
                    onclick: move |_| {
                        if !media_controls_enabled {
                            return;
                        }
                        let send_realtime = camera_realtime.clone();
                        let send_server_id = camera_server_id.clone();
                        let send_room_id = camera_room_id.clone();
                        let local_user_id = camera_user_id.clone();
                        let local_video = camera_video.clone();
                        toggle_camera.toggle(Rc::new(move |frame| {
                            let frame_realtime = send_realtime.clone();
                            let frame_server_id = send_server_id.clone();
                            let frame_room_id = send_room_id.clone();
                            local_video.publish_frame(
                                ParticipantVideoSource::Camera,
                                ParticipantVideoFrame::from_local_camera(
                                    frame_room_id.clone(),
                                    local_user_id.clone(),
                                    frame.clone(),
                                ),
                            );
                            spawn(async move {
                                if let Err(error) = realtime::send_camera_frame(
                                    &frame_realtime,
                                    &frame_server_id,
                                    &frame_room_id,
                                    frame,
                                ).await {
                                    warn!(%error, server_id = %frame_server_id, room_id = %frame_room_id, "failed to send encoded direct-call camera frame");
                                }
                            });
                        }));
                    },
                    span { class: control_icon_class(camera_live), {camera_icon(false)} }
                    span { class: control_icon_class(!camera_live), {camera_icon(true)} }
                }
                AndroidOutputRouteButton { enabled: media_controls_enabled }
                div { class: "direct-call-inline-secondary shrink-0 items-center gap-1.5",
                    {screen_share_button(
                        screen_share_live,
                        screen_share_starting,
                        media_controls_enabled,
                        screen_share_label,
                        false,
                        {
                            let toggle_screen_share_handle = toggle_screen_share_handle.clone();
                            let screen_share_realtime = screen_share_realtime.clone();
                            let screen_share_server_id = screen_share_server_id.clone();
                            let screen_share_room_id = screen_share_room_id.clone();
                            move |_| {
                                debug!(room_id = %screen_share_room_id, "toggling direct-call screen share from primary controls");
                                toggle_screen_share(
                                    toggle_screen_share_handle.clone(),
                                    screen_share_realtime.clone(),
                                    screen_share_server_id.clone(),
                                    screen_share_room_id.clone(),
                                );
                            }
                        },
                    )}
                    {chat_button(chat_open, false, move |_| on_toggle_chat.call(()))}
                }
                div { class: "direct-call-overflow relative shrink-0",
                    if more_open() {
                        button {
                            r#type: "button",
                            class: "pointer-events-auto fixed inset-0 z-0 cursor-default bg-transparent",
                            "aria-label": "Закрыть дополнительные действия",
                            onclick: move |_| {
                                more_open.set(false);
                                controls_visibility.set_locked(false);
                            },
                        }
                        div {
                            id: "direct-call-more-menu",
                            class: "direct-call-more-menu pointer-events-auto absolute bottom-[calc(100%+10px)] left-1/2 z-30 flex -translate-x-1/2 items-center gap-1.5 rounded-[18px] bg-zinc-900/94 p-1.5 shadow-[0_0_0_1px_rgba(255,255,255,.10),0_16px_44px_rgba(0,0,0,.42)] backdrop-blur-xl",
                            role: "menu",
                            {screen_share_button(
                                screen_share_live,
                                screen_share_starting,
                                media_controls_enabled,
                                screen_share_label,
                                true,
                                move |_| {
                                    debug!(room_id = %screen_share_room_id, "toggling direct-call screen share from more menu");
                                    more_open.set(false);
                                    controls_visibility.set_locked(false);
                                    toggle_screen_share(
                                        toggle_screen_share_handle.clone(),
                                        screen_share_realtime.clone(),
                                        screen_share_server_id.clone(),
                                        screen_share_room_id.clone(),
                                    );
                                },
                            )}
                            {chat_button(chat_open, true, move |_| {
                                on_toggle_chat.call(());
                                more_open.set(false);
                                controls_visibility.set_locked(false);
                            })}
                        }
                    }
                    button {
                        r#type: "button",
                        class: more_button_class,
                        "aria-label": "Дополнительные действия",
                        "aria-controls": "direct-call-more-menu",
                        "aria-expanded": if more_open() { "true" } else { "false" },
                        onclick: move |_| {
                            let next_open = !more_open();
                            debug!(open = next_open, "toggling direct-call more menu");
                            more_open.set(next_open);
                            controls_visibility.set_locked(next_open);
                        },
                        svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", d: "M5 12h.01M12 12h.01M19 12h.01" }
                        }
                    }
                }
                div { class: "mx-0.5 h-9 w-px shrink-0 bg-white/[0.08]" }
                button {
                    r#type: "button",
                    disabled: is_leaving,
                    class: "voice-control-button direct-call-control group relative flex h-12 w-12 items-center justify-center rounded-xl bg-red-500 text-white shadow-[0_10px_26px_rgba(239,68,68,.22)] transition-[scale,background-color,box-shadow,opacity] duration-[180ms] hover:bg-red-400 active:scale-[0.96] disabled:cursor-wait disabled:opacity-60",
                    "aria-label": "Завершить звонок",
                    onclick: move |_| {
                        info!(room_id = %leave_target.room_id, "ending direct call from primary control dock");
                        leave_microphone.stop();
                        leave_camera.stop();
                        leave_screen_share.stop();
                        if !leave_direct_call.end_for_target(&leave_target) {
                            voice.leave();
                        }
                    },
                    svg { class: "h-5 w-5 rotate-[135deg]", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M2.25 6.75c0 8.284 6.716 15 15 15h2.25a2.25 2.25 0 0 0 2.25-2.25v-1.372c0-.516-.351-.966-.852-1.091l-4.423-1.106c-.44-.11-.902.055-1.173.417l-.97 1.293c-.282.376-.769.542-1.21.38a12.035 12.035 0 0 1-7.143-7.143c-.162-.441.004-.928.38-1.21l1.293-.97c.362-.271.527-.734.417-1.173L6.963 3.102A1.125 1.125 0 0 0 5.872 2.25H4.5A2.25 2.25 0 0 0 2.25 4.5v2.25Z" }
                    }
                }
            }
        }
    }
}

fn toggle_screen_share(
    screen_share: ScreenShareHandle,
    realtime_handle: RealtimeHandle,
    server_id: String,
    room_id: String,
) {
    screen_share.toggle(Rc::new(move |frame| {
        let frame_realtime = realtime_handle.clone();
        let frame_server_id = server_id.clone();
        let frame_room_id = room_id.clone();
        spawn(async move {
            if let Err(error) = realtime::send_screen_frame(
                &frame_realtime,
                &frame_server_id,
                &frame_room_id,
                frame,
            ).await {
                warn!(%error, server_id = %frame_server_id, room_id = %frame_room_id, "failed to send encoded direct-call screen frame");
            }
        });
    }));
}

fn chat_button(
    active: bool,
    menu_item: bool,
    onclick: impl FnMut(MouseEvent) + 'static,
) -> Element {
    let label = if active {
        "Скрыть текстовый чат"
    } else {
        "Открыть текстовый чат"
    };
    let class = if active {
        "voice-control-button direct-call-control group relative flex h-12 w-12 items-center justify-center rounded-xl bg-blue-500/16 text-blue-100 shadow-[0_0_0_1px_rgba(96,165,250,.30)] transition-[scale,background-color,box-shadow] duration-[180ms] active:scale-[0.96]"
    } else {
        "voice-control-button direct-call-control group relative flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-950/65 text-zinc-200 shadow-[0_0_0_1px_rgba(255,255,255,.08)] transition-[scale,background-color,box-shadow] duration-[180ms] active:scale-[0.96]"
    };

    rsx! {
        button {
            r#type: "button",
            class,
            role: if menu_item { "menuitem" } else { "button" },
            "aria-label": label,
            "aria-pressed": active,
            title: label,
            onclick,
            svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M7 18.5 3.5 21v-4.6A8.5 8.5 0 1 1 7 18.5Z" }
                path { stroke_linecap: "round", d: "M8 10h8M8 14h5" }
            }
        }
    }
}

fn screen_share_button(
    live: bool,
    starting: bool,
    enabled: bool,
    label: &'static str,
    menu_item: bool,
    onclick: impl FnMut(MouseEvent) + 'static,
) -> Element {
    let class = if live {
        "voice-control-button direct-call-control group relative flex h-12 w-12 items-center justify-center rounded-xl bg-blue-500/16 text-blue-100 shadow-[0_0_0_1px_rgba(96,165,250,.30)] transition-[scale,background-color,box-shadow,opacity] duration-[180ms] active:scale-[0.96] disabled:cursor-wait disabled:opacity-60"
    } else {
        "voice-control-button direct-call-control group relative flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-950/65 text-zinc-200 shadow-[0_0_0_1px_rgba(255,255,255,.08)] transition-[scale,background-color,box-shadow,opacity] duration-[180ms] active:scale-[0.96] disabled:cursor-wait disabled:opacity-60"
    };
    rsx! {
        button {
            r#type: "button",
            disabled: !enabled || starting,
            class,
            role: if menu_item { "menuitem" } else { "button" },
            "aria-label": label,
            title: label,
            onclick,
            span { class: control_icon_class(!live), {screen_share_icon(false)} }
            span { class: control_icon_class(live), {screen_share_icon(true)} }
        }
    }
}

fn control_icon_class(visible: bool) -> &'static str {
    if visible {
        "pointer-events-none relative z-10 grid scale-100 place-items-center opacity-100 blur-0 transition-[transform,opacity,filter] duration-300 ease-[cubic-bezier(0.2,0,0,1)]"
    } else {
        "pointer-events-none absolute z-10 grid scale-[0.25] place-items-center opacity-0 blur-[4px] transition-[transform,opacity,filter] duration-300 ease-[cubic-bezier(0.2,0,0,1)]"
    }
}

fn microphone_icon(muted: bool) -> Element {
    rsx! { svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
        if muted {
            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 9.34V7a3 3 0 0 0-5.68-1.34M9 9v2a3 3 0 0 0 5.12 2.12M19 11a7 7 0 0 1-7 7m0 0v3m0-3a7 7 0 0 1-7-7m3 10h8M3 3l18 18" }
        } else {
            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11a7 7 0 0 1-14 0m7 8v3m-4 0h8m-4-18a3 3 0 0 0-3 3v4a3 3 0 1 0 6 0V7a3 3 0 0 0-3-3Z" }
        }
    }}
}

fn camera_icon(off: bool) -> Element {
    rsx! { svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
        if off {
            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3 3l18 18M15 10l4.55-2.28A1 1 0 0 1 21 8.62v6.76a1 1 0 0 1-.4.8M15 10v1.5M15 14a2 2 0 0 1-2 2H6.5M5 8h8a2 2 0 0 1 1.5.68M3 10v4a2 2 0 0 0 2 2h1" }
        } else {
            path { stroke_linecap: "round", stroke_linejoin: "round", d: "m15 10 4.55-2.28A1 1 0 0 1 21 8.62v6.76a1 1 0 0 1-1.45.9L15 14m0-4v4m0-4a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2" }
        }
    }}
}

fn screen_share_icon(stopping: bool) -> Element {
    rsx! { svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
        rect { x: "3", y: "4", width: "18", height: "12", rx: "2" }
        if stopping {
            rect { x: "9", y: "8", width: "6", height: "5", rx: "1" }
            path { stroke_linecap: "round", d: "M8 20h8m-4-4v4" }
        } else {
            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 20h8m-4-4v-9m0 0-3 3m3-3 3 3" }
        }
    }}
}

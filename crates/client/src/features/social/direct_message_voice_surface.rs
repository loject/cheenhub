//! Голосовая область выбранного личного диалога.

use cheenhub_contracts::realtime::{DirectCallEndReason, DirectCallState};
use cheenhub_contracts::rest::DmConversationSummary;
use dioxus::prelude::*;

use crate::features::app::components::avatar::UserAvatar;
use crate::features::app::current_user::CurrentUserContext;
use crate::features::microphone::{MicrophoneHandle, MicrophoneStatus};
use crate::features::runtime::sleep_ms;
use crate::features::voice_chat::{
    DirectCallControls, DirectCallControlsVisibility, DirectCallHandle, DirectCallStage,
    DirectCallStageStatus, DirectCallUiState, VoiceConnectionHandle, VoiceConnectionState,
};
use web_time::Instant;

use super::voice_target::direct_message_voice_target;

/// Рендерит активный голосовой звонок выбранного личного диалога.
#[component]
pub(crate) fn DirectMessageVoiceSurface(
    conversation: DmConversationSummary,
    exiting: bool,
    chat_open: bool,
    on_toggle_chat: EventHandler<()>,
) -> Element {
    let current_user = use_context::<CurrentUserContext>().require_user();
    let voice = use_context::<VoiceConnectionHandle>();
    let direct_call = use_context::<DirectCallHandle>();
    let microphone = use_context::<MicrophoneHandle>();
    let controls_visible = use_signal(|| true);
    let controls_locked = use_signal(|| false);
    let controls_last_activity = use_signal(Instant::now);
    let controls_visibility = DirectCallControlsVisibility::new(
        controls_visible,
        controls_locked,
        controls_last_activity,
    );
    use_context_provider(|| controls_visibility);
    use_future(move || async move {
        loop {
            sleep_ms(250).await;
            controls_visibility.hide_if_idle();
        }
    });
    let target = direct_message_voice_target(&conversation);
    let call = direct_call.call_for_conversation(&conversation.id);
    let starting = direct_call.is_starting_conversation(&conversation.id);
    let signaling_error = direct_call.error_for_conversation(&conversation.id);
    let voice_state = voice.state();
    let selected_voice_active = voice_state
        .active_target()
        .is_some_and(|active| active.matches(&target));
    let selected_voice_participants = if selected_voice_active {
        voice_state.participants().to_vec()
    } else {
        Vec::new()
    };
    let peer_present = selected_voice_participants
        .iter()
        .any(|participant| participant.user_id == conversation.friend_user_id);
    let selected_voice_status = match &voice_state {
        VoiceConnectionState::Connecting {
            target: connecting_target,
        } if connecting_target.matches(&target) => DirectCallStageStatus::Connecting,
        VoiceConnectionState::Connected {
            target: connected_target,
            ..
        } if connected_target.matches(&target) && peer_present => DirectCallStageStatus::Connected,
        VoiceConnectionState::Connected {
            target: connected_target,
            ..
        } if connected_target.matches(&target) => DirectCallStageStatus::WaitingForPeer,
        VoiceConnectionState::Error {
            target: error_target,
            message,
        } if error_target
            .as_ref()
            .is_some_and(|error_target| error_target.matches(&target)) =>
        {
            DirectCallStageStatus::Error {
                message: message.clone(),
            }
        }
        _ => DirectCallStageStatus::Recovering,
    };
    let mut selected_voice_speaking_user_ids = if selected_voice_active {
        voice.speaking_user_ids()
    } else {
        Vec::new()
    };
    if selected_voice_active
        && matches!(microphone.status(), MicrophoneStatus::Live)
        && microphone.level_active()
        && !selected_voice_speaking_user_ids
            .iter()
            .any(|user_id| user_id == &current_user.id)
    {
        selected_voice_speaking_user_ids.push(current_user.id.clone());
    }
    let retry_voice = voice.clone();
    let outgoing_ringing = call.as_ref().is_some_and(|call| {
        call.state == DirectCallState::Ringing && direct_call.is_outgoing(call)
    });
    let incoming_ringing = call.as_ref().is_some_and(|call| {
        call.state == DirectCallState::Ringing && !direct_call.is_outgoing(call)
    });
    let ended = call
        .as_ref()
        .is_some_and(|call| call.state == DirectCallState::Ended);
    let waiting = starting || outgoing_ringing || incoming_ringing || ended;
    let active_signaling_error = selected_voice_active
        .then(|| signaling_error.clone())
        .flatten();
    let waiting_title = if starting {
        "Начинаем звонок"
    } else if outgoing_ringing {
        "Звоним"
    } else if incoming_ringing {
        "Входящий звонок"
    } else if let Some(call) = call.as_ref() {
        match call.end_reason {
            Some(DirectCallEndReason::Declined) => "Звонок отклонён",
            Some(DirectCallEndReason::TimedOut) => "Нет ответа",
            Some(DirectCallEndReason::Cancelled) => "Звонок отменён",
            _ => "Звонок завершён",
        }
    } else {
        "Личный звонок"
    };
    let waiting_body = if starting || outgoing_ringing {
        format!("Ожидаем ответ от {}", conversation.friend_nickname)
    } else if incoming_ringing {
        format!("{} хочет поговорить с тобой", conversation.friend_nickname)
    } else {
        "Можно вернуться к переписке или позвонить снова.".to_owned()
    };
    let cancel_call = direct_call.clone();
    let accept_call = direct_call.clone();
    let decline_call = direct_call.clone();
    let retry_call = direct_call.clone();
    let retry_target = target.clone();

    rsx! {
        div {
            class: if exiting {
                "direct-call-surface direct-call-surface-exit voice-room-surface relative flex min-h-0 flex-1 flex-col"
            } else {
                "direct-call-surface voice-room-surface relative flex min-h-0 flex-1 flex-col"
            },
            onclick: move |_| controls_visibility.reveal(),
            onfocusin: move |_| controls_visibility.reveal(),
            onmousemove: move |_| controls_visibility.reveal(),
            if waiting || signaling_error.is_some() && !selected_voice_active {
                div { class: "direct-call-stage voice-stage flex min-h-0 flex-1 items-center justify-center overflow-y-auto p-5 pb-[112px]",
                    div { class: "direct-call-waiting-content w-full max-w-sm text-center",
                        UserAvatar {
                            nickname: conversation.friend_nickname.clone(),
                            avatar_url: conversation.friend_avatar_url.clone(),
                            class: if starting || outgoing_ringing || incoming_ringing {
                                "direct-call-avatar direct-call-avatar-ringing mx-auto flex h-24 w-24 items-center justify-center rounded-full bg-zinc-900 text-[28px] font-bold text-zinc-50 shadow-[0_0_0_1px_rgba(255,255,255,.10),0_18px_50px_rgba(0,0,0,.30)]".to_owned()
                            } else {
                                "direct-call-avatar mx-auto flex h-24 w-24 items-center justify-center rounded-full bg-zinc-900 text-[28px] font-bold text-zinc-50 shadow-[0_0_0_1px_rgba(255,255,255,.10),0_18px_50px_rgba(0,0,0,.30)]".to_owned()
                            },
                            avatar_seed: Some(conversation.friend_user_id.clone()),
                        }
                        div { class: "direct-call-enter-segment direct-call-enter-segment-copy",
                            h2 { class: "mt-5 text-balance text-[20px] font-semibold text-zinc-50", "{waiting_title}" }
                            p { class: "mt-2 text-pretty text-[13px] leading-6 text-zinc-400", "{waiting_body}" }
                        }
                        if let Some(message) = signaling_error {
                            p {
                                class: "direct-call-status-enter mx-auto mt-4 max-w-xs rounded-xl bg-red-500/10 px-3 py-2 text-pretty text-[12px] leading-5 text-red-200 shadow-[0_0_0_1px_rgba(248,113,113,.22)]",
                                role: "alert",
                                "{message}"
                            }
                        }
                        if outgoing_ringing {
                            button {
                                r#type: "button",
                                disabled: direct_call.busy(),
                                class: "direct-call-enter-segment direct-call-enter-segment-actions mt-6 inline-flex min-h-12 items-center justify-center rounded-2xl bg-red-500/12 px-5 text-[13px] font-semibold text-red-100 shadow-[0_0_0_1px_rgba(248,113,113,.28)] transition-[scale,background-color,opacity] duration-150 ease-out active:scale-[0.96] disabled:cursor-wait disabled:opacity-60",
                                onclick: move |_| cancel_call.cancel(),
                                "Отменить звонок"
                            }
                        } else if incoming_ringing {
                            div { class: "direct-call-enter-segment direct-call-enter-segment-actions mx-auto mt-6 grid max-w-xs grid-cols-2 gap-3",
                                button {
                                    r#type: "button",
                                    disabled: direct_call.busy(),
                                    class: "min-h-12 rounded-2xl bg-red-500/12 px-4 text-[13px] font-semibold text-red-100 shadow-[0_0_0_1px_rgba(248,113,113,.28)] transition-[scale,background-color,opacity] duration-150 ease-out active:scale-[0.96] disabled:cursor-wait disabled:opacity-60",
                                    onclick: move |_| decline_call.decline(),
                                    "Отклонить"
                                }
                                button {
                                    r#type: "button",
                                    disabled: direct_call.busy(),
                                    class: "min-h-12 rounded-2xl bg-accent px-4 text-[13px] font-semibold text-white transition-[scale,background-color,opacity] duration-150 ease-out active:scale-[0.96] disabled:cursor-wait disabled:opacity-60",
                                    onclick: move |_| accept_call.accept(),
                                    "Ответить"
                                }
                            }
                        } else if matches!(
                            direct_call.state(),
                            DirectCallUiState::Error { call: None, .. }
                        ) {
                            button {
                                r#type: "button",
                                disabled: direct_call.busy(),
                                class: "direct-call-enter-segment direct-call-enter-segment-actions mt-6 inline-flex min-h-12 items-center justify-center rounded-2xl bg-accent px-5 text-[13px] font-semibold text-white transition-[scale,background-color,opacity] duration-150 ease-out active:scale-[0.96] disabled:cursor-wait disabled:opacity-60",
                                onclick: move |_| retry_call.start(retry_target.clone()),
                                "Позвонить снова"
                            }
                        }
                    }
                }
            } else {
                if let Some(message) = active_signaling_error {
                    p {
                        class: "direct-call-status-enter absolute left-1/2 top-4 z-30 w-[min(90%,360px)] -translate-x-1/2 rounded-xl bg-red-950/95 px-3 py-2 text-center text-pretty text-[12px] leading-5 text-red-100 shadow-[0_0_0_1px_rgba(248,113,113,.28),0_12px_32px_rgba(0,0,0,.30)]",
                        role: "alert",
                        "{message}"
                    }
                }
                DirectCallStage {
                    peer_nickname: conversation.friend_nickname.clone(),
                    peer_avatar_url: conversation.friend_avatar_url.clone(),
                    peer_user_id: conversation.friend_user_id.clone(),
                    current_user_id: current_user.id.clone(),
                    participants: selected_voice_participants,
                    speaking_user_ids: selected_voice_speaking_user_ids,
                    status: selected_voice_status,
                    on_retry: {
                        let retry_target = target.clone();
                        move |_| retry_voice.join(retry_target.clone())
                    },
                }
                DirectCallControls {
                    target,
                    chat_open,
                    on_toggle_chat,
                }
            }
        }
    }
}

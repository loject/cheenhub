//! Кнопка входа в голосовой звонок личного диалога.

use cheenhub_contracts::rest::DmConversationSummary;
use dioxus::prelude::*;

use cheenhub_contracts::realtime::{DirectCallEndReason, DirectCallState};

use crate::features::voice_chat::{DirectCallHandle, DirectCallUiState};

use super::voice_target::direct_message_voice_target;

/// Рендерит кнопку входа в голосовой звонок выбранного личного диалога.
#[component]
pub(crate) fn DirectMessageVoiceButton(conversation: DmConversationSummary) -> Element {
    let direct_call = use_context::<DirectCallHandle>();
    let target = direct_message_voice_target(&conversation);
    let conversation_id = conversation.id.clone();
    let call = direct_call.call_for_conversation(&conversation_id);
    let starting = direct_call.is_starting_conversation(&conversation_id);
    let busy = direct_call.busy();
    let incoming = call.as_ref().is_some_and(|call| {
        call.state == DirectCallState::Ringing && !direct_call.is_outgoing(call)
    });
    let active = call
        .as_ref()
        .is_some_and(|call| call.state == DirectCallState::Active);
    let ringing = call
        .as_ref()
        .is_some_and(|call| call.state == DirectCallState::Ringing);
    let another_call_open = call.is_none()
        && !matches!(
            direct_call.state(),
            DirectCallUiState::Idle | DirectCallUiState::Error { call: None, .. }
        );
    let label = if starting {
        "Начинаем звонок..."
    } else if incoming {
        "Ответить"
    } else if ringing {
        "Звоним..."
    } else if active {
        "Звонок идёт"
    } else if let Some(call) = call
        .as_ref()
        .filter(|call| call.state == DirectCallState::Ended)
    {
        match call.end_reason {
            Some(DirectCallEndReason::Declined) => "Звонок отклонён",
            Some(DirectCallEndReason::TimedOut) => "Нет ответа",
            _ => "Звонок завершён",
        }
    } else if another_call_open {
        "Другой звонок активен"
    } else {
        "Начать звонок"
    };
    let highlighted = incoming || ringing || active;
    let disabled = busy || starting || active || ringing && !incoming || another_call_open;
    let action_call = direct_call.clone();

    rsx! {
        button {
            r#type: "button",
            disabled,
            class: if highlighted {
                "flex min-h-12 shrink-0 items-center gap-2 rounded-xl bg-emerald-500/12 px-3 text-[12px] font-medium text-emerald-100 shadow-[0_0_0_1px_rgba(52,211,153,.28)] transition-[scale,background-color,box-shadow,opacity] duration-150 ease-out active:scale-[0.96] disabled:cursor-wait disabled:opacity-70 max-[420px]:w-12 max-[420px]:justify-center max-[420px]:px-0"
            } else {
                "flex min-h-12 shrink-0 items-center gap-2 rounded-xl bg-zinc-900/80 px-3 text-[12px] font-medium text-zinc-200 shadow-[0_0_0_1px_rgba(255,255,255,.08)] transition-[scale,background-color,box-shadow,opacity] duration-150 ease-out active:scale-[0.96] disabled:cursor-wait disabled:opacity-70 max-[420px]:w-12 max-[420px]:justify-center max-[420px]:px-0"
            },
            "aria-label": label,
            onclick: move |_| {
                if incoming {
                    action_call.accept();
                    return;
                }
                if call.is_none() {
                    action_call.start(target.clone());
                }
            },
            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M2.25 6.75c0 8.284 6.716 15 15 15h2.25a2.25 2.25 0 0 0 2.25-2.25v-1.372c0-.516-.351-.966-.852-1.091l-4.423-1.106c-.44-.11-.902.055-1.173.417l-.97 1.293c-.282.376-.769.542-1.21.38a12.035 12.035 0 0 1-7.143-7.143c-.162-.441.004-.928.38-1.21l1.293-.97c.362-.271.527-.734.417-1.173L6.963 3.102A1.125 1.125 0 0 0 5.872 2.25H4.5A2.25 2.25 0 0 0 2.25 4.5v2.25Z" }
            }
            span { class: "max-[420px]:sr-only", "{label}" }
        }
    }
}

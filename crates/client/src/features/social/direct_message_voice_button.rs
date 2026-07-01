//! Кнопка входа в голосовой звонок личного диалога.

use cheenhub_contracts::rest::DmConversationSummary;
use dioxus::prelude::*;

use crate::features::voice_chat::{VoiceConnectionHandle, VoiceConnectionState};

use super::voice_target::direct_message_voice_target;

/// Рендерит кнопку входа в голосовой звонок выбранного личного диалога.
#[component]
pub(crate) fn DirectMessageVoiceButton(conversation: DmConversationSummary) -> Element {
    let voice = use_context::<VoiceConnectionHandle>();
    let target = direct_message_voice_target(&conversation);
    let voice_state = voice.state();
    let selected_voice_active = voice_state
        .active_target()
        .is_some_and(|active| active.matches(&target));
    let selected_voice_busy = matches!(
        voice_state,
        VoiceConnectionState::Connecting { .. } | VoiceConnectionState::Disconnecting { .. }
    );
    let selected_voice_join_label = if selected_voice_busy && selected_voice_active {
        "Подключаемся..."
    } else if selected_voice_active {
        "Звонок открыт"
    } else {
        "Начать звонок"
    };

    rsx! {
        button {
            r#type: "button",
            disabled: selected_voice_busy,
            class: if selected_voice_active {
                "flex h-9 shrink-0 items-center gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3 text-[12px] font-medium text-emerald-100 transition hover:border-emerald-400/40 hover:bg-emerald-500/15 disabled:cursor-wait disabled:opacity-70"
            } else {
                "flex h-9 shrink-0 items-center gap-2 rounded-lg border border-zinc-800 bg-zinc-900/80 px-3 text-[12px] font-medium text-zinc-200 transition hover:border-zinc-700 hover:bg-zinc-900 disabled:cursor-wait disabled:opacity-70"
            },
            "aria-label": selected_voice_join_label,
            onclick: move |_| {
                if voice
                    .state()
                    .active_target()
                    .is_some_and(|active| active.matches(&target))
                {
                    return;
                }
                voice.join(target.clone());
            },
            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M2.25 6.75c0 8.284 6.716 15 15 15h2.25a2.25 2.25 0 0 0 2.25-2.25v-1.372c0-.516-.351-.966-.852-1.091l-4.423-1.106c-.44-.11-.902.055-1.173.417l-.97 1.293c-.282.376-.769.542-1.21.38a12.035 12.035 0 0 1-7.143-7.143c-.162-.441.004-.928.38-1.21l1.293-.97c.362-.271.527-.734.417-1.173L6.963 3.102A1.125 1.125 0 0 0 5.872 2.25H4.5A2.25 2.25 0 0 0 2.25 4.5v2.25Z" }
            }
            "{selected_voice_join_label}"
        }
    }
}

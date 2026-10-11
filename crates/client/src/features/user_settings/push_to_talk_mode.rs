//! Карточка режима удержания, согласованная с другими вариантами активации.

use crate::features::microphone::{MicrophoneActivationMode, MicrophoneHandle, push_to_talk};
use dioxus::prelude::*;

/// Выбирает Push-to-talk, не размещая дополнительные настройки внутри grid карточек.
#[component]
pub(super) fn PushToTalkMode() -> Element {
    let mic = use_context::<MicrophoneHandle>();
    let supported = push_to_talk::supported();
    let selected = mic.activation_mode() == MicrophoneActivationMode::PushToTalk;
    rsx! {
        button { r#type: "button", disabled: !supported, aria_pressed: selected,
            class: if supported { super::sound_section::activation_button_class(selected) } else { "rounded-xl border border-zinc-800 bg-zinc-900/40 p-3 text-left opacity-50 cursor-not-allowed" },
            onclick: move |_| mic.set_activation_mode(MicrophoneActivationMode::PushToTalk),
            div { class: "font-medium", "Push-to-talk" }
            div { class: "mt-1 text-[12px] leading-4 text-zinc-400", if supported { "Говорите, удерживая кнопку клавиатуры или мыши." } else { "Недоступно на этой платформе." } }
        }
    }
}

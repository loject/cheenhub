//! Карточка режима удержания, согласованная с другими вариантами активации.

use crate::features::microphone::{MicrophoneActivationMode, MicrophoneHandle, push_to_talk};
use dioxus::prelude::*;

/// Выбирает Push-to-talk, не размещая дополнительные настройки внутри grid карточек.
#[component]
pub(super) fn PushToTalkMode() -> Element {
    let mic = use_context::<MicrophoneHandle>();
    let available = use_resource(push_to_talk::availability);
    let update = mic.push_to_talk_update();
    let pending = available().is_none()
        || update
            .as_ref()
            .is_some_and(|update| update.availability == push_to_talk::Availability::Pending);
    let supported = update.map_or(available().unwrap_or(false), |update| {
        update.availability == push_to_talk::Availability::Available
    });
    let selected = mic.activation_mode() == MicrophoneActivationMode::PushToTalk;
    rsx! {
        button { r#type: "button", disabled: !supported, aria_pressed: selected,
            class: if supported { super::sound_section::activation_button_class(selected) } else { "rounded-xl border border-zinc-800 bg-zinc-900/40 p-3 text-left opacity-50 cursor-not-allowed" },
            onclick: move |_| mic.set_activation_mode(MicrophoneActivationMode::PushToTalk),
            div { class: "font-medium", "Push-to-talk" }
            div { class: "mt-1 text-[12px] leading-4 text-zinc-400", if pending { "Проверяем доступность…" } else if supported { {push_to_talk::mode_description()} } else { "Недоступно в этой среде." } }
        }
    }
}

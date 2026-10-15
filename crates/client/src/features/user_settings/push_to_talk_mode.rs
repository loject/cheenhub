//! Карточка режима удержания, согласованная с другими вариантами активации.

use crate::features::microphone::{MicrophoneActivationMode, MicrophoneHandle, push_to_talk};
use dioxus::prelude::*;

/// Показывает режим удержания и локальный статус его доступности.
#[component]
pub(super) fn PushToTalkMode() -> Element {
    let mic = use_context::<MicrophoneHandle>();
    let available = use_resource(push_to_talk::availability);
    let update = mic.push_to_talk_update();
    let pending = update.as_ref().map_or(available().is_none(), |update| {
        update.availability.is_none()
    });
    let supported = update.map_or(available().unwrap_or(false), |update| {
        update.availability == Some(true)
    });
    let selected = mic.activation_mode() == MicrophoneActivationMode::PushToTalk;
    rsx! {
        button { r#type: "button", disabled: !supported, aria_pressed: selected,
            class: super::sound_section::activation_button_class(selected),
            onclick: move |_| mic.set_activation_mode(MicrophoneActivationMode::PushToTalk),
            div { class: if supported { "font-medium" } else { "font-medium text-zinc-400" }, "Push-to-talk" }
            div { class: "mt-1 text-[12px] leading-4 text-zinc-400",
                if pending { "Проверяем доступность…" }
                else if supported { {push_to_talk::mode_description()} }
                else { "Требуется настройка" }
            }
        }
    }
}

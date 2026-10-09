//! Назначение кнопки удержания с загрузкой, отменой и обработкой ошибок.

use crate::features::microphone::{MicrophoneHandle, push_to_talk};
use dioxus::core::Task;
use dioxus::prelude::*;

/// Показывает запись кнопки отдельной областью под карточками режимов.
#[component]
pub(super) fn PushToTalkSettings() -> Element {
    let mic = use_context::<MicrophoneHandle>();
    let mut task = use_signal(|| None::<Task>);
    let mut error = use_signal(|| None::<String>);
    let busy = task().is_some();
    let key = mic.push_to_talk_key();
    let record_mic = mic.clone();
    if !push_to_talk::supported() {
        return rsx! { p { class: "mt-3 text-[12px] leading-5 text-zinc-400", {push_to_talk::unsupported_reason()} } };
    }
    rsx! {
        div { class: "mt-4 space-y-3 rounded-2xl border border-zinc-800 bg-zinc-900/45 p-4",
            div { class: "flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between",
                div { class: "min-w-0",
                    h4 { class: "text-[14px] font-semibold text-zinc-100", "Кнопка разговора" }
                    p { class: "mt-1 text-[12px] leading-5 text-zinc-400", "Работает и когда приложение свёрнуто." }
                }
                div { class: "flex flex-wrap items-center gap-2",
                    span { class: "inline-flex min-h-10 items-center rounded-xl border border-zinc-700 bg-zinc-950 px-4 text-[13px] font-medium text-zinc-100", {key.label()} }
                    button { r#type: "button", disabled: busy,
                        class: "inline-flex min-h-10 items-center justify-center gap-2 rounded-xl border border-accent/30 bg-accent/10 px-4 text-[13px] font-medium text-zinc-100 transition hover:border-blue-400/45 disabled:cursor-wait",
                        onclick: move |_| {
                            if task().is_some() { return; }
                            error.set(None);
                            let microphone = record_mic.clone();
                            let pending = spawn(async move {
                                let result = microphone.record_push_to_talk_binding().await;
                                if let Err(message) = result { warn!("push-to-talk binding recording did not complete"); error.set(Some(message)); }
                                task.set(None);
                            });
                            task.set(Some(pending));
                        },
                        if busy {
                            span { aria_hidden: "true", class: "h-4 w-4 animate-spin rounded-full border-2 border-zinc-600 border-t-blue-300" }
                            "Ожидаем кнопку…"
                        } else { "Назначить" }
                    }
                    if busy {
                        button { r#type: "button", class: "min-h-10 rounded-xl px-3 text-[13px] text-zinc-300 transition hover:bg-zinc-800",
                            onpointerdown: move |event| { event.prevent_default(); if let Some(pending) = task.take() { pending.cancel(); } error.set(None); info!("push-to-talk binding recording cancelled"); },
                            onclick: move |_| { if let Some(pending) = task.take() { pending.cancel(); } error.set(None); },
                            "Отменить"
                        }
                    }
                }
            }
            if busy {
                p { role: "status", class: "rounded-xl bg-zinc-950/60 px-3 py-2 text-[13px] leading-5 text-zinc-200", "Нажмите и отпустите клавишу или кнопку мыши, включая Mouse4 и Mouse5. Передача голоса приостановлена на время назначения." }
            } else {
                p { class: "text-[12px] leading-5 text-zinc-400", "Удерживайте выбранную кнопку, чтобы говорить. Отпустите, чтобы прекратить передачу. Кнопка также продолжит работать в других приложениях." }
            }
            if let Some(message) = error() { p { role: "alert", class: "text-[12px] leading-5 text-red-300", "{message}" } }
        }
    }
}

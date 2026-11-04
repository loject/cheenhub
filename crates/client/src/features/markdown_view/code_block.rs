//! Отображение и копирование исходного кода сообщения.

use super::highlight::highlight;
use crate::features::{clipboard::copy_text, runtime::sleep_ms};
use dioxus::prelude::*;

/// Показывает блок кода с ограниченной областью прокрутки.
#[component]
pub(super) fn MarkdownCodeBlock(language: Option<String>, code: String) -> Element {
    let highlighted = use_memo(use_reactive((&language, &code), |(language, code)| {
        highlight(language.as_deref(), &code)
    }));
    let mut status = use_signal(|| 0_u8);
    let label = language.unwrap_or_else(|| "Код".into());
    rsx! {
        div { class: "my-2 min-w-0 overflow-hidden rounded-lg bg-black/10 dark:bg-white/10",
            div { class: "flex items-center justify-between gap-3 px-3 py-2 text-xs",
                span { class: "font-mono opacity-70", "{label}" }
                button {
                    r#type: "button",
                    class: "rounded px-2 py-1 hover:bg-white/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-400",
                    disabled: status() == 1,
                    onkeydown: move |event| event.stop_propagation(),
                    onclick: move |event| {
                        event.stop_propagation();
                        let code = code.clone();
                        status.set(1);
                        spawn(async move {
                            match copy_text(code).await {
                                Ok(()) => { status.set(2); debug!("code block copied"); }
                                Err(error) => { status.set(3); warn!(%error, "failed to copy code block"); }
                            }
                            sleep_ms(2000).await;
                            status.set(0);
                        });
                    },
                    match status() { 1 => "Копируем…", 2 => "Скопировано", 3 => "Не удалось скопировать", _ => "Копировать" }
                }
            }
            pre { class: "max-h-[400px] overflow-auto whitespace-pre p-3 font-mono text-[0.9em]", code {
                for (index, token) in highlighted().into_iter().enumerate() {
                    span { key: "{index}", class: "{token.class}", "{token.text}" }
                }
            } }
        }
    }
}

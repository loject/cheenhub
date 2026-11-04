//! Разбор текста сообщения и отображение результата.

use dioxus::prelude::*;

use super::{cache::parse_cached, node::MarkdownNode};

/// Отображает полный текст сообщения с безопасным текстовым fallback.
#[component]
pub(crate) fn MarkdownContent(
    body: String,
    #[props(default)] message_id: Option<String>,
) -> Element {
    let parsed = use_memo(use_reactive((&message_id, &body), |(message_id, body)| {
        parse_cached(message_id.as_deref(), &body)
    }));
    let nodes = match parsed() {
        Ok(nodes) => nodes,
        Err(error) => {
            warn!(?error, "failed to parse message markdown");
            return rsx! { span { "Не удалось отобразить сообщение" } };
        }
    };
    rsx! {
        div { class: "min-w-0 [overflow-wrap:anywhere]",
            for (index, node) in nodes.iter().cloned().enumerate() {
                MarkdownNode { key: "{index}", node }
            }
        }
    }
}

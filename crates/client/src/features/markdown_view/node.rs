//! Рекурсивное представление узлов сообщения.

use cheenhub_client_worker::markdown::{MessageNode, TableAlignment};
use dioxus::prelude::*;

use super::code_block::MarkdownCodeBlock;
use super::spoiler::MarkdownSpoiler;

fn safe_url(value: &str) -> Option<String> {
    let url = url::Url::parse(value).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host_str().is_some()).then(|| url.to_string())
}

/// Рендерит узел с сохранением структуры и вложенности.
#[component]
pub(super) fn MarkdownNode(node: MessageNode) -> Element {
    match node {
        MessageNode::Text(text) => rsx! { "{text}" },
        MessageNode::Paragraph(children) => {
            rsx! { div { class: "whitespace-pre-wrap", for (index, node) in children.into_iter().enumerate() { MarkdownNode { key: "{index}", node } } } }
        }
        MessageNode::Strong(children) => {
            rsx! { strong { for (index, node) in children.into_iter().enumerate() { MarkdownNode { key: "{index}", node } } } }
        }
        MessageNode::Emphasis(children) => {
            rsx! { em { for (index, node) in children.into_iter().enumerate() { MarkdownNode { key: "{index}", node } } } }
        }
        MessageNode::Underline(children) => {
            rsx! { u { for (index, node) in children.into_iter().enumerate() { MarkdownNode { key: "{index}", node } } } }
        }
        MessageNode::Strikethrough(children) => {
            rsx! { s { for (index, node) in children.into_iter().enumerate() { MarkdownNode { key: "{index}", node } } } }
        }
        MessageNode::Spoiler(children) => rsx! { MarkdownSpoiler { nodes: children } },
        MessageNode::InlineCode(code) => {
            rsx! { code { class: "rounded bg-black/10 px-1 font-mono text-[0.9em] dark:bg-white/10", "{code}" } }
        }
        MessageNode::CodeBlock { language, code } => rsx! { MarkdownCodeBlock { language, code } },
        MessageNode::Heading { level, children } => {
            let class = match level {
                1 => "text-2xl font-bold",
                2 => "text-xl font-bold",
                _ => "text-lg font-semibold",
            };
            rsx! { div { class: "{class}", for (index, node) in children.into_iter().enumerate() { MarkdownNode { key: "{index}", node } } } }
        }
        MessageNode::Quote(children) => {
            rsx! { blockquote { class: "my-1 border-l-2 border-current pl-3 opacity-80", for (index, node) in children.into_iter().enumerate() { MarkdownNode { key: "{index}", node } } } }
        }
        MessageNode::List {
            ordered,
            start,
            items,
        } => {
            if ordered {
                rsx! { ol { class: "my-1 list-decimal pl-6", start: "{start.unwrap_or(1)}", for (index, item) in items.into_iter().enumerate() { li { key: "{index}", for (child_index, node) in item.into_iter().enumerate() { MarkdownNode { key: "{child_index}", node } } } } } }
            } else {
                rsx! { ul { class: "my-1 list-disc pl-6", for (index, item) in items.into_iter().enumerate() { li { key: "{index}", for (child_index, node) in item.into_iter().enumerate() { MarkdownNode { key: "{child_index}", node } } } } } }
            }
        }
        MessageNode::Link { url, children } => {
            if let Some(url) = safe_url(&url) {
                rsx! { a { href: "{url}", target: "_blank", rel: "noopener noreferrer", class: "text-blue-500 underline dark:text-blue-300", onclick: move |event| event.stop_propagation(), onkeydown: move |event| event.stop_propagation(), for (index, node) in children.into_iter().enumerate() { MarkdownNode { key: "{index}", node } } } }
            } else {
                rsx! { span { title: "{url}", for (index, node) in children.into_iter().enumerate() { MarkdownNode { key: "{index}", node } } } }
            }
        }
        MessageNode::Table { alignments, rows } => rsx! {
            div { class: "my-2 max-w-full overflow-x-auto",
                table { class: "w-max border-collapse text-left",
                    for (row_index, row) in rows.into_iter().enumerate() {
                        tr { key: "{row_index}",
                            for (index, cell) in row.cells.into_iter().enumerate() {
                                td {
                                    key: "{index}",
                                    class: if row.header { "border border-zinc-700/50 px-2 py-1 font-semibold" } else { "border border-zinc-700/50 px-2 py-1" },
                                    style: match alignments.get(index) { Some(TableAlignment::Left) => "text-align:left", Some(TableAlignment::Center) => "text-align:center", Some(TableAlignment::Right) => "text-align:right", _ => "" },
                                    for (child_index, node) in cell.into_iter().enumerate() { MarkdownNode { key: "{child_index}", node } }
                                }
                            }
                        }
                    }
                }
            }
        },
    }
}

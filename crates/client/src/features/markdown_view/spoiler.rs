//! Локальное состояние раскрытия одного спойлера.

use cheenhub_client_worker::markdown::MessageNode;
use dioxus::prelude::*;

use super::node::MarkdownNode;

/// Скрывает содержимое до явного раскрытия пользователем.
#[component]
pub(super) fn MarkdownSpoiler(nodes: Vec<MessageNode>) -> Element {
    let mut revealed = use_signal(|| false);
    let width = visible_length(&nodes).clamp(3, 40);
    rsx! {
        span {
            role: "button",
            tabindex: "0",
            "aria-expanded": if revealed() { "true" } else { "false" },
            "aria-label": if revealed() { "Скрыть спойлер" } else { "Показать спойлер" },
            class: if revealed() { "cursor-pointer rounded bg-zinc-500/15 hover:bg-zinc-500/25 focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-400" } else { "markdown-spoiler-grain inline-block h-[1em] max-w-full cursor-pointer select-none rounded-sm bg-zinc-500 align-[-0.12em] hover:bg-zinc-400 focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-400" },
            style: if revealed() { String::new() } else { format!("width: {width}ch") },
            onclick: move |event| {
                event.stop_propagation();
                revealed.toggle();
                debug!(revealed = revealed(), "message spoiler visibility changed");
            },
            onkeydown: move |event| {
                if event.key() == Key::Named(NamedKey::Enter) || event.key() == Key::Character(" ".into()) {
                    event.prevent_default();
                    event.stop_propagation();
                    revealed.toggle();
                    debug!(revealed = revealed(), "message spoiler visibility changed");
                }
            },
            if revealed() {
                for (index, node) in nodes.into_iter().enumerate() {
                    MarkdownNode { key: "{index}", node }
                }
            }
        }
    }
}

fn visible_length(nodes: &[MessageNode]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            MessageNode::Text(text) | MessageNode::InlineCode(text) => text.chars().count(),
            MessageNode::CodeBlock { code, .. } => code.chars().count(),
            MessageNode::Paragraph(children)
            | MessageNode::Strong(children)
            | MessageNode::Emphasis(children)
            | MessageNode::Underline(children)
            | MessageNode::Strikethrough(children)
            | MessageNode::Spoiler(children)
            | MessageNode::Quote(children)
            | MessageNode::Heading { children, .. }
            | MessageNode::Link { children, .. } => visible_length(children),
            MessageNode::List { items, .. } => items.iter().map(|item| visible_length(item)).sum(),
            MessageNode::Table { rows, .. } => rows
                .iter()
                .flat_map(|row| &row.cells)
                .map(|cell| visible_length(cell))
                .sum(),
        })
        .sum()
}

//! Текстовое представление кратких превью без раскрытия спойлеров.

use super::cache::parse_cached;
use cheenhub_client_worker::markdown::MessageNode;

/// Формирует краткий текст для неинтерактивного превью сообщения.
///
/// Скрытое содержимое не попадает в результат даже при ограничении числа строк.
pub(crate) fn markdown_preview(body: &str, message_id: Option<&str>) -> String {
    match parse_cached(message_id, body) {
        Ok(nodes) => {
            let mut output = String::new();
            append(&nodes, &mut output);
            output
        }
        Err(error) => {
            dioxus::logger::tracing::warn!(?error, "failed to parse message markdown preview");
            "Не удалось отобразить сообщение".into()
        }
    }
}

fn append(nodes: &[MessageNode], output: &mut String) {
    for node in nodes {
        match node {
            MessageNode::Text(text) | MessageNode::InlineCode(text) => output.push_str(text),
            MessageNode::CodeBlock { code, .. } => output.push_str(code),
            MessageNode::Spoiler(_) => output.push_str("Спойлер"),
            MessageNode::Paragraph(children) => {
                append(children, output);
                output.push(' ');
            }
            MessageNode::Strong(children)
            | MessageNode::Emphasis(children)
            | MessageNode::Underline(children)
            | MessageNode::Strikethrough(children)
            | MessageNode::Quote(children)
            | MessageNode::Heading { children, .. }
            | MessageNode::Link { children, .. } => append(children, output),
            MessageNode::List { items, .. } => {
                for item in items {
                    append(item, output);
                    output.push(' ');
                }
            }
            MessageNode::Table { rows, .. } => {
                for row in rows {
                    for cell in &row.cells {
                        append(cell, output);
                        output.push(' ');
                    }
                }
            }
        }
    }
}

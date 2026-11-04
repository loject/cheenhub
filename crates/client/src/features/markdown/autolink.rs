//! Выделение адресов в текстовых узлах без изменения исходного Markdown.

use super::MessageNode;

pub(super) fn split(text: &str) -> Vec<MessageNode> {
    let mut result = Vec::new();
    let mut cursor = 0;
    for (start, part) in text.split_inclusive(char::is_whitespace).scan(0, |offset, part| {
        let start = *offset;
        *offset += part.len();
        Some((start, part))
    }) {
        let lead = part.find(|c: char| !matches!(c, '(' | '[' | '{' | '«' | '"' | '\'')).unwrap_or(part.len());
        let candidate = &part[lead..];
        let mut end = candidate.trim_end_matches(|c: char| c.is_whitespace() || matches!(c, '.' | ',' | '!' | '?' | ';' | ':' | '»' | '"' | '\'' | ']' | '}')).len();
        while candidate[..end].ends_with(')') && candidate[..end].matches(')').count() > candidate[..end].matches('(').count() {
            end -= 1;
        }
        let value = &candidate[..end];
        let explicit = value.starts_with("https://") || value.starts_with("http://");
        let host = value.strip_prefix("https://").or_else(|| value.strip_prefix("http://")).unwrap_or(value).split(['/', '?', '#', ':']).next().unwrap_or("");
        if value.contains('@') || value.contains('\\') || value.chars().any(char::is_control)
            || (!explicit && value.contains("://")) || !host.contains('.')
            || host.starts_with('.') || host.ends_with('.') || host.split('.').any(|label| label.is_empty() || label.starts_with('-') || label.ends_with('-') || !label.chars().all(|c| c.is_alphanumeric() || c == '-'))
        { continue; }
        let begin = start + lead;
        if begin > cursor { result.push(MessageNode::Text(text[cursor..begin].to_owned())); }
        result.push(MessageNode::Link {
            url: if explicit { value.to_owned() } else { format!("https://{value}") },
            children: vec![MessageNode::Text(value.to_owned())],
        });
        cursor = begin + end;
    }
    if cursor < text.len() { result.push(MessageNode::Text(text[cursor..].to_owned())); }
    result
}

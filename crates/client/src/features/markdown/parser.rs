//! Преобразование AST Comrak в независимую модель сообщения.

use std::cell::Cell;

use comrak::{
    Arena, Options,
    nodes::{AstNode, ListType, NodeValue, Sourcepos},
    try_parse_document,
};

use super::{MessageNode, TableAlignment, TableRow};

const MAX_BYTES: usize = 65_536;
const MAX_NODES: usize = 8_192;
const MAX_DEPTH: usize = 64;

/// Причина отказа в разборе; исходное сообщение остаётся у вызывающего кода.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Превышен предел размера в 65536 байт.
    InputTooLarge,
    /// Превышен предел количества узлов или глубины дерева.
    TooComplex,
    /// Позиция исходного фрагмента, возвращённая парсером, некорректна.
    InvalidSourcePosition,
}

/// Разбирает недоверенное сообщение в дерево без выполнения содержимого.
///
/// При превышении ограничений дерево целиком отклоняется, чтобы вызывающий
/// код мог отобразить исходное сообщение как текст.
///
/// # Errors
/// Возвращает причину превышения ограничения размера или сложности.
pub fn parse_message(input: &str) -> Result<Vec<MessageNode>, ParseError> {
    if input.len() > MAX_BYTES {
        tracing::warn!(bytes = input.len(), "markdown input exceeds size limit");
        return Err(ParseError::InputTooLarge);
    }
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.underline = true;
    options.extension.spoiler = true;
    let arena = Arena::new();
    let checks = Cell::new(0_usize);
    let root = try_parse_document(&arena, input, &options, &|| {
        checks.set(checks.get() + 1);
        checks.get() > 128
    })
    .map_err(|_| {
        tracing::warn!("markdown parsing exceeds work limit");
        ParseError::TooComplex
    })?;
    let mut lines = vec![0];
    lines.extend(input.match_indices('\n').map(|(offset, _)| offset + 1));
    let mut converter = Converter {
        input,
        lines,
        count: 0,
    };
    let result = converter.children(root, 0, false);
    if result.is_err() {
        tracing::warn!(
            nodes = converter.count,
            error = ?result.as_ref().err(),
            "markdown conversion failed"
        );
    }
    result
}

struct Converter<'a> {
    input: &'a str,
    lines: Vec<usize>,
    count: usize,
}

impl Converter<'_> {
    fn children<'a>(
        &mut self,
        node: &'a AstNode<'a>,
        depth: usize,
        quote: bool,
    ) -> Result<Vec<MessageNode>, ParseError> {
        let mut result = Vec::new();
        for child in node.children() {
            result.extend(self.node(child, depth + 1, quote)?);
        }
        Ok(result)
    }

    fn node<'a>(
        &mut self,
        node: &'a AstNode<'a>,
        depth: usize,
        quote: bool,
    ) -> Result<Vec<MessageNode>, ParseError> {
        self.count += 1;
        if self.count > MAX_NODES || depth > MAX_DEPTH {
            return Err(ParseError::TooComplex);
        }
        let data = node.data.borrow();
        let wrap = |value| Ok(vec![value]);
        match &data.value {
            NodeValue::Text(text) => wrap(MessageNode::Text(text.to_string())),
            NodeValue::SoftBreak | NodeValue::LineBreak => wrap(MessageNode::Text("\n".into())),
            NodeValue::Code(code) => wrap(MessageNode::InlineCode(code.literal.clone())),
            NodeValue::CodeBlock(code) => wrap(MessageNode::CodeBlock {
                language: code.info.split_whitespace().next().map(str::to_owned),
                code: code.literal.clone(),
            }),
            NodeValue::Paragraph => {
                wrap(MessageNode::Paragraph(self.children(node, depth, quote)?))
            }
            NodeValue::Strong => wrap(MessageNode::Strong(self.children(node, depth, quote)?)),
            NodeValue::Emph if self.follows_escaped_asterisk(data.sourcepos) => {
                wrap(MessageNode::Text(self.source(data.sourcepos)?))
            }
            NodeValue::Emph => wrap(MessageNode::Emphasis(self.children(node, depth, quote)?)),
            NodeValue::Underline => {
                wrap(MessageNode::Underline(self.children(node, depth, quote)?))
            }
            NodeValue::Strikethrough => wrap(MessageNode::Strikethrough(
                self.children(node, depth, quote)?,
            )),
            NodeValue::SpoileredText => {
                wrap(MessageNode::Spoiler(self.children(node, depth, quote)?))
            }
            NodeValue::Heading(heading) if heading.level <= 3 => wrap(MessageNode::Heading {
                level: heading.level,
                children: self.children(node, depth, quote)?,
            }),
            NodeValue::BlockQuote if !quote => {
                wrap(MessageNode::Quote(self.children(node, depth, true)?))
            }
            NodeValue::BlockQuote => self.children(node, depth, true),
            NodeValue::List(list) => {
                let ordered = list.list_type == ListType::Ordered;
                let mut items = Vec::new();
                for item in node.children() {
                    self.count += 1;
                    if self.count > MAX_NODES {
                        return Err(ParseError::TooComplex);
                    }
                    items.push(self.children(item, depth + 1, quote)?);
                }
                wrap(MessageNode::List {
                    ordered,
                    start: ordered.then_some(list.start as u64),
                    items,
                })
            }
            NodeValue::Link(link) => wrap(MessageNode::Link {
                url: link.url.clone(),
                children: self.children(node, depth, quote)?,
            }),
            NodeValue::Table(table) => {
                let alignments = table
                    .alignments
                    .iter()
                    .map(|a| match a {
                        comrak::nodes::TableAlignment::None => TableAlignment::None,
                        comrak::nodes::TableAlignment::Left => TableAlignment::Left,
                        comrak::nodes::TableAlignment::Center => TableAlignment::Center,
                        comrak::nodes::TableAlignment::Right => TableAlignment::Right,
                    })
                    .collect();
                let mut rows = Vec::new();
                for row in node.children() {
                    let header = matches!(row.data.borrow().value, NodeValue::TableRow(true));
                    let mut cells = Vec::new();
                    for cell in row.children() {
                        self.count += 2;
                        if self.count > MAX_NODES || depth + 2 > MAX_DEPTH {
                            return Err(ParseError::TooComplex);
                        }
                        cells.push(self.children(cell, depth + 2, quote)?);
                    }
                    rows.push(TableRow { header, cells });
                }
                wrap(MessageNode::Table { alignments, rows })
            }
            NodeValue::HtmlInline(html) => wrap(MessageNode::Text(html.clone())),
            NodeValue::HtmlBlock(html) => wrap(MessageNode::Text(html.literal.clone())),
            _ => wrap(MessageNode::Text(self.source(data.sourcepos)?)),
        }
    }

    fn source(&self, pos: Sourcepos) -> Result<String, ParseError> {
        let start = self
            .lines
            .get(pos.start.line.saturating_sub(1))
            .and_then(|n| n.checked_add(pos.start.column.saturating_sub(1)));
        let end = self
            .lines
            .get(pos.end.line.saturating_sub(1))
            .and_then(|n| n.checked_add(pos.end.column));
        start
            .zip(end)
            .and_then(|(a, b)| self.input.get(a..b))
            .map(str::to_owned)
            .ok_or(ParseError::InvalidSourcePosition)
    }

    fn follows_escaped_asterisk(&self, pos: Sourcepos) -> bool {
        let Some(&start) = self.lines.get(pos.start.line.saturating_sub(1)) else {
            return false;
        };
        let offset = pos.start.column.saturating_sub(1);
        let Some(end) = start.checked_add(offset) else {
            return false;
        };
        let Some(prefix) = self.input.as_bytes().get(start..end) else {
            return false;
        };
        if !prefix.ends_with(b"\\*") {
            return false;
        }
        let slashes = prefix[..prefix.len() - 1]
            .iter()
            .rev()
            .take_while(|byte| **byte == b'\\')
            .count();
        slashes % 2 == 1
    }
}

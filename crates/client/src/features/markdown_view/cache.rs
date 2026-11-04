//! Кеш деревьев Markdown сообщений для повторных отображений.

use cheenhub_client_worker::markdown::{MessageNode, ParseError, parse_message};
use std::{cell::RefCell, collections::VecDeque, mem::size_of, rc::Rc};

const MAX_ENTRIES: usize = 128;
const MAX_BYTES: usize = 4 * 1024 * 1024;

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::default());
}

#[derive(Default)]
struct Cache {
    entries: VecDeque<Entry>,
    bytes: usize,
}

struct Entry {
    id: String,
    body: String,
    nodes: Rc<Vec<MessageNode>>,
    bytes: usize,
}

impl Cache {
    fn get(&mut self, id: &str, body: &str) -> Option<Rc<Vec<MessageNode>>> {
        let index = self.entries.iter().position(|entry| entry.id == id)?;
        let entry = self.entries.remove(index)?;
        if entry.body != body {
            self.bytes -= entry.bytes;
            return None;
        }
        let nodes = entry.nodes.clone();
        self.entries.push_back(entry);
        Some(nodes)
    }

    fn insert(&mut self, id: &str, body: &str, nodes: Rc<Vec<MessageNode>>) {
        let bytes = size_of::<Entry>()
            + id.len()
            + body.len()
            + size_of::<Vec<MessageNode>>()
            + 2 * size_of::<usize>()
            + nodes_size(&nodes, nodes.capacity());
        if bytes > MAX_BYTES {
            return;
        }
        while self.entries.len() >= MAX_ENTRIES || self.bytes + bytes > MAX_BYTES {
            if let Some(entry) = self.entries.pop_front() {
                self.bytes -= entry.bytes;
            } else {
                break;
            }
        }
        self.bytes += bytes;
        self.entries.push_back(Entry {
            id: id.to_owned(),
            body: body.to_owned(),
            nodes,
            bytes,
        });
    }
}

fn nodes_size(nodes: &[MessageNode], capacity: usize) -> usize {
    capacity * size_of::<MessageNode>()
        + nodes
            .iter()
            .map(|node| match node {
                MessageNode::Text(text) | MessageNode::InlineCode(text) => text.capacity(),
                MessageNode::CodeBlock { language, code } => {
                    language.as_ref().map_or(0, String::capacity) + code.capacity()
                }
                MessageNode::Paragraph(children)
                | MessageNode::Strong(children)
                | MessageNode::Emphasis(children)
                | MessageNode::Underline(children)
                | MessageNode::Strikethrough(children)
                | MessageNode::Spoiler(children)
                | MessageNode::Quote(children)
                | MessageNode::Heading { children, .. } => {
                    nodes_size(children, children.capacity())
                }
                MessageNode::Link { url, children } => {
                    url.capacity() + nodes_size(children, children.capacity())
                }
                MessageNode::List { items, .. } => {
                    items.capacity() * size_of::<Vec<MessageNode>>()
                        + items
                            .iter()
                            .map(|item| nodes_size(item, item.capacity()))
                            .sum::<usize>()
                }
                MessageNode::Table { alignments, rows } => {
                    alignments.capacity()
                        * size_of::<cheenhub_client_worker::markdown::TableAlignment>()
                        + rows.capacity() * size_of::<cheenhub_client_worker::markdown::TableRow>()
                        + rows
                            .iter()
                            .map(|row| {
                                row.cells.capacity() * size_of::<Vec<MessageNode>>()
                                    + row
                                        .cells
                                        .iter()
                                        .map(|cell| nodes_size(cell, cell.capacity()))
                                        .sum::<usize>()
                            })
                            .sum::<usize>()
                }
            })
            .sum::<usize>()
}

/// Возвращает дерево Markdown для указанной версии сообщения.
pub(super) fn parse_cached(
    id: Option<&str>,
    body: &str,
) -> Result<Rc<Vec<MessageNode>>, ParseError> {
    let id = id.filter(|id| !id.is_empty());
    if let Some(nodes) = id.and_then(|id| CACHE.with_borrow_mut(|cache| cache.get(id, body))) {
        return Ok(nodes);
    }
    let nodes = Rc::new(parse_message(body)?);
    if let Some(id) = id {
        CACHE.with_borrow_mut(|cache| cache.insert(id, body, nodes.clone()));
    }
    Ok(nodes)
}

#[cfg(test)]
mod tests;

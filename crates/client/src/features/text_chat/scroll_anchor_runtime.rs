//! Измерение и восстановление якоря средствами Dioxus в одном экземпляре чата.

use std::{collections::HashMap, rc::Rc};

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::dioxus_elements::geometry::PixelsVector2D;
use dioxus::prelude::*;

use super::scroll::{ScrollCommand, apply_scroll_command};
use super::scroll_anchor::{ScrollAnchor, choose_anchor, restored_offset};

/// Смонтированные сообщения и версия прокрутки локального списка.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct AnchorElements {
    rows: Signal<HashMap<String, Rc<MountedData>>>,
    revision: Signal<u64>,
}

impl AnchorElements {
    /// Отмечает ручное взаимодействие со списком, отменяя устаревшее восстановление.
    pub(super) fn interacted(mut self) {
        let next = self.revision.peek().wrapping_add(1);
        self.revision.set(next);
    }

    /// Проверяет, не переместил ли пользователь список после измерения.
    pub(super) fn is_current(self, anchor: &ScrollAnchor) -> bool {
        *self.revision.peek() == anchor.revision
    }
}

/// Создаёт реестр только для текущей комнаты и обрабатывает команды прокрутки.
pub(super) fn use_anchor_elements(
    list: Signal<Option<Rc<MountedData>>>,
    mut pending: Signal<Option<ScrollCommand>>,
    messages: Signal<Vec<TextChatMessage>>,
    mut layout: Signal<super::VirtualChatLayout>,
) -> AnchorElements {
    let rows = use_signal(HashMap::new);
    let revision = use_signal(|| 0);
    let elements = AnchorElements { rows, revision };
    let mut restoring = use_signal(|| false);
    use_effect(move || {
        let command = pending();
        let _mounted_rows = rows.read();
        let _revision = revision();
        let current_ids = messages
            .read()
            .iter()
            .map(|message| message.id.clone())
            .collect::<Vec<_>>();
        if restoring() {
            return;
        }
        let Some(command) = command else { return };
        let Some(list) = list.cloned() else { return };
        if let ScrollCommand::Restore { ref anchor } = command {
            if !elements.is_current(anchor) {
                pending.set(None);
                debug!(message_id = %anchor.message_id, "cancelled stale text chat scroll anchor");
                return;
            }
            let Some((message_id, inside_y)) = anchor.resolve(&current_ids) else {
                pending.set(None);
                return;
            };
            if message_id != anchor.message_id {
                let mut replacement = anchor.clone();
                replacement.message_id = message_id;
                replacement.inside_y = inside_y;
                pending.set(Some(ScrollCommand::Restore {
                    anchor: replacement,
                }));
                return;
            }
            if !rows.peek().contains_key(&anchor.message_id) {
                return;
            }
        }
        restoring.set(true);
        spawn(async move {
            match &command {
                ScrollCommand::Restore { anchor } => {
                    if restore_anchor(list, elements, anchor).await
                        && let Some((group_id, _, _, _)) =
                            super::prepare_text_chat_groups(&messages.peek())
                                .into_iter()
                                .find(|(_, _, _, group)| {
                                    group.iter().any(|message| message.id == anchor.message_id)
                                })
                    {
                        layout.write().restore_row(group_id);
                    }
                }
                _ => apply_scroll_command(list, command.clone()).await,
            }
            if pending.peek().as_ref() == Some(&command) {
                let ids = messages
                    .peek()
                    .iter()
                    .map(|message| message.id.clone())
                    .collect::<Vec<_>>();
                pending.set(remaining_restore(command, &ids));
            }
            restoring.set(false);
        });
    });
    elements
}

/// Регистрирует элемент сообщения и удаляет только его собственное измерение при unmount.
pub(super) fn use_message_anchor(
    message_id: String,
    elements: Option<AnchorElements>,
) -> EventHandler<Rc<MountedData>> {
    let mut own_element = use_signal(|| None::<Rc<MountedData>>);
    let drop_id = message_id.clone();
    use_drop(move || {
        if let Some(mut elements) = elements
            && let Some(own) = own_element.peek().as_ref()
        {
            let mut rows = elements.rows.write();
            if rows.get(&drop_id).is_some_and(|row| Rc::ptr_eq(row, own)) {
                rows.remove(&drop_id);
            }
        }
    });
    use_callback(move |element: Rc<MountedData>| {
        if let Some(mut elements) = elements {
            own_element.set(Some(element.clone()));
            elements.rows.write().insert(message_id.clone(), element);
        }
    })
}

/// Измеряет сообщение у верхней границы окна непосредственно перед заменой истории.
pub(super) async fn capture_anchor(
    list: Rc<MountedData>,
    elements: AnchorElements,
    messages: &[TextChatMessage],
) -> Option<ScrollAnchor> {
    let revision = *elements.revision.peek();
    let viewport = list.get_client_rect().await.ok()?;
    let mounted = elements.rows.peek().clone();
    let mut rows = Vec::new();
    for message in messages {
        if let Some(element) = mounted.get(&message.id)
            && let Ok(rect) = element.get_client_rect().await
        {
            rows.push((message.id.clone(), rect.origin.y, rect.size.height));
            if rect.origin.y + rect.size.height > viewport.origin.y
                && rect.origin.y < viewport.origin.y + viewport.size.height
            {
                break;
            }
        }
    }
    if revision != *elements.revision.peek() {
        return None;
    }
    let (message_id, inside_y) = choose_anchor(
        &rows,
        viewport.origin.y,
        viewport.origin.y + viewport.size.height,
    )?;
    let index = messages
        .iter()
        .position(|message| message.id == message_id)?;
    Some(ScrollAnchor {
        message_id,
        inside_y,
        preceding_ids: messages[..index]
            .iter()
            .rev()
            .map(|message| message.id.clone())
            .collect(),
        revision,
    })
}

/// Принудительно монтирует группу с целевым сообщением, даже если её placeholder вне окна.
pub(super) fn targets_group(command: Option<ScrollCommand>, messages: &[TextChatMessage]) -> bool {
    match command {
        Some(ScrollCommand::Restore { anchor }) => messages
            .iter()
            .any(|message| message.id == anchor.message_id),
        _ => false,
    }
}

/// Повторяет восстановление, если сообщение удалили во время асинхронного измерения.
fn remaining_restore(command: ScrollCommand, ids: &[String]) -> Option<ScrollCommand> {
    let ScrollCommand::Restore { mut anchor } = command else {
        return None;
    };
    if ids.contains(&anchor.message_id) {
        return None;
    }
    let (message_id, inside_y) = anchor.resolve(ids)?;
    debug!(deleted_message_id = %anchor.message_id, next_message_id = %message_id,
        "retargeted text chat scroll anchor after deletion during measurement");
    anchor.message_id = message_id;
    anchor.inside_y = inside_y;
    Some(ScrollCommand::Restore { anchor })
}

async fn restore_anchor(
    list: Rc<MountedData>,
    elements: AnchorElements,
    anchor: &ScrollAnchor,
) -> bool {
    let Some(row) = elements.rows.peek().get(&anchor.message_id).cloned() else {
        return false;
    };
    let measurements = async {
        let viewport = list.get_client_rect().await?;
        let rect = row.get_client_rect().await?;
        let offset = list.get_scroll_offset().await?;
        Ok::<_, dioxus::html::MountedError>((viewport, rect, offset))
    }
    .await;
    let Ok((viewport, rect, offset)) = measurements else {
        warn!(message_id = %anchor.message_id, "failed to measure text chat scroll anchor");
        return false;
    };
    if !elements.is_current(anchor) {
        return false;
    }
    let y = restored_offset(
        offset.y,
        rect.origin.y,
        viewport.origin.y,
        rect.size.height,
        anchor.inside_y,
    );
    if let Err(error) = list
        .scroll(PixelsVector2D::new(0.0, y), ScrollBehavior::Instant)
        .await
    {
        warn!(?error, message_id = %anchor.message_id, "failed to restore text chat scroll anchor");
        false
    } else {
        debug!(message_id = %anchor.message_id, inside_y = anchor.inside_y, "restored text chat scroll anchor");
        true
    }
}

#[cfg(test)]
#[path = "scroll_anchor_runtime_tests.rs"]
mod tests;

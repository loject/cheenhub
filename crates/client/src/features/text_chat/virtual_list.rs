//! Виртуализация тяжелых строк истории чата с сохранением их места в раскладке.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;

use super::{friendly_message_date, group_consecutive_messages, message_day_key};

const OVERSCAN_ROWS: usize = 2;
const INITIAL_RENDER_ROWS: usize = 4;
const SMALL_LIST_ROWS: usize = 4;
const DEFAULT_ROW_HEIGHT: f64 = 96.0;
const MIN_ROW_HEIGHT: f64 = 1.0;
const HEIGHT_CHANGE_EPSILON: f64 = 0.5;
const IMAGE_PREVIEW_MAX_WIDTH: f64 = 520.0;
const IMAGE_PREVIEW_MAX_HEIGHT: f64 = 360.0;
const IMAGE_PREVIEW_FALLBACK_HEIGHT: f64 = 210.0;

pub(crate) type VirtualTextChatGroup = (String, Option<String>, f64, Vec<TextChatMessage>);

/// Подготавливает устойчивые ключи, разделители дат и начальные высоты групп комнаты.
pub(crate) fn prepare_text_chat_groups(messages: &[TextChatMessage]) -> Vec<VirtualTextChatGroup> {
    let mut previous_day_key = None;
    group_consecutive_messages(messages)
        .into_iter()
        .filter_map(|group| {
            let first_message = group.first()?;
            let day_key = message_day_key(&first_message.created_at);
            let date_label = (previous_day_key.as_ref() != Some(&day_key))
                .then(|| friendly_message_date(&first_message.created_at));
            previous_day_key = Some(day_key);
            let estimated_height = estimated_group_height(
                group.iter().map(|message| message.body.chars().count()),
                group.iter().flat_map(|message| {
                    message.attachments.iter().map(|attachment| {
                        estimated_image_preview_height(attachment.width, attachment.height)
                    })
                }),
            );
            Some((
                first_message.id.clone(),
                date_label,
                estimated_height,
                group,
            ))
        })
        .collect()
}

/// Измерения и видимость строк одного экземпляра истории.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct VirtualChatLayout {
    visible_rows: HashSet<String>,
    measured_heights: HashMap<String, f64>,
    anchor_row: Option<String>,
}

impl VirtualChatLayout {
    /// Возвращает непрерывный диапазон строк, чьи тяжелые дочерние компоненты должны жить в DOM.
    pub(crate) fn rendered_range(&self, ordered_rows: &[String]) -> Range<usize> {
        if ordered_rows.len() <= SMALL_LIST_ROWS {
            return 0..ordered_rows.len();
        }

        let mut visible_indices = ordered_rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| self.visible_rows.contains(row).then_some(index));
        let first_visible = visible_indices.next();
        let last_visible = visible_indices.next_back().or(first_visible);

        let (first, last) = match (first_visible, last_visible) {
            (Some(first), Some(last)) => (first, last),
            _ => self
                .anchor_row
                .as_ref()
                .and_then(|anchor| ordered_rows.iter().position(|row| row == anchor))
                .map(|index| (index, index))
                .unwrap_or_else(|| {
                    let first = ordered_rows.len().saturating_sub(INITIAL_RENDER_ROWS);
                    (first, ordered_rows.len().saturating_sub(1))
                }),
        };

        first.saturating_sub(OVERSCAN_ROWS)
            ..last
                .saturating_add(OVERSCAN_ROWS + 1)
                .min(ordered_rows.len())
    }

    /// Удерживает восстановленную группу до получения новых событий видимости.
    pub(super) fn restore_row(&mut self, row: String) {
        self.visible_rows.clear();
        self.visible_rows.insert(row.clone());
        self.anchor_row = Some(row);
    }

    /// Обновляет видимость строки, сохраняя последний вошедший в viewport якорь.
    fn set_visible(&mut self, row: &str, visible: bool) {
        if visible {
            self.visible_rows.insert(row.to_owned());
            self.anchor_row = Some(row.to_owned());
        } else {
            self.visible_rows.remove(row);
        }
    }

    /// Запоминает только валидное измерение и сообщает, изменилась ли раскладка существенно.
    fn record_height(&mut self, row: &str, height: f64) -> bool {
        if !height.is_finite() || height < MIN_ROW_HEIGHT {
            return false;
        }
        if self
            .measured_heights
            .get(row)
            .is_some_and(|saved| (saved - height).abs() <= HEIGHT_CHANGE_EPSILON)
        {
            return false;
        }
        self.measured_heights.insert(row.to_owned(), height);
        true
    }

    fn placeholder_height(&self, row: &str, estimated_height: f64) -> f64 {
        self.measured_heights
            .get(row)
            .copied()
            .unwrap_or_else(|| sanitize_estimated_height(estimated_height))
    }

    fn remove_row(&mut self, row: &str) {
        self.visible_rows.remove(row);
        self.measured_heights.remove(row);
        if self.anchor_row.as_deref() == Some(row) {
            self.anchor_row = None;
        }
    }
}

/// Обертка оставляет легкий placeholder точной высоты, когда тяжелая строка размонтирована.
#[component]
pub(crate) fn VirtualChatRow(
    row_id: String,
    active: bool,
    estimated_height: f64,
    mut layout: Signal<VirtualChatLayout>,
    children: Element,
) -> Element {
    let cleanup_row_id = row_id.clone();
    use_drop(move || layout.write().remove_row(&cleanup_row_id));

    let inactive_style = (!active).then(|| {
        let height = layout.read().placeholder_height(&row_id, estimated_height);
        format!("height: {height:.2}px; overflow: hidden; contain: layout paint;")
    });
    let visible_row_id = row_id.clone();
    let measured_row_id = row_id.clone();

    rsx! {
        div {
            class: "virtual-chat-row min-w-0",
            "aria-hidden": (!active).then_some("true"),
            "data-virtual-chat-active": if active { "true" } else { "false" },
            style: inactive_style,
            onvisible: move |event| {
                let Ok(visible) = event.is_intersecting() else {
                    return;
                };
                let mut saved = layout.write();
                saved.set_visible(&visible_row_id, visible);
            },
            onresize: move |event| {
                if !active {
                    return;
                }
                let Ok(size) = event.get_border_box_size() else {
                    return;
                };
                let mut saved = layout.write();
                saved.record_height(&measured_row_id, size.height);
            },
            if active {
                {children}
            }
        }
    }
}

/// Оценка высоты превью совпадает с ограничениями обоих компонентов изображений чата.
pub(crate) fn estimated_image_preview_height(width: i32, height: i32) -> f64 {
    if width <= 0 || height <= 0 {
        return IMAGE_PREVIEW_FALLBACK_HEIGHT;
    }
    let width = f64::from(width);
    let height = f64::from(height);
    let scale = (IMAGE_PREVIEW_MAX_WIDTH / width)
        .min(IMAGE_PREVIEW_MAX_HEIGHT / height)
        .min(1.0);
    (height * scale)
        .round()
        .clamp(1.0, IMAGE_PREVIEW_MAX_HEIGHT)
}

/// Дает достаточно близкую начальную высоту до первого реального `onresize`.
pub(crate) fn estimated_group_height(
    message_bodies: impl IntoIterator<Item = usize>,
    image_heights: impl IntoIterator<Item = f64>,
) -> f64 {
    let bodies = message_bodies.into_iter().collect::<Vec<_>>();
    let message_count = bodies.len().max(1);
    let body_height = bodies
        .into_iter()
        .map(|characters| {
            if characters == 0 {
                0.0
            } else {
                let lines = characters.div_ceil(72).clamp(1, 12);
                16.0 + lines as f64 * 20.0
            }
        })
        .sum::<f64>();
    let images = image_heights
        .into_iter()
        .map(|height| height + 8.0)
        .sum::<f64>();
    let message_gaps = message_count.saturating_sub(1) as f64 * 8.0;

    // Шапка автора, отметки времени и межстрочные интервалы группы.
    44.0 + body_height + images + message_count as f64 * 16.0 + message_gaps
}

fn sanitize_estimated_height(height: f64) -> f64 {
    if height.is_finite() && height >= MIN_ROW_HEIGHT {
        height
    } else {
        DEFAULT_ROW_HEIGHT
    }
}

#[cfg(test)]
mod tests;

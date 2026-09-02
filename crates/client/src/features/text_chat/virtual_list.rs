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
mod tests {
    use std::cell::Cell;

    use cheenhub_contracts::realtime::{TextChatImageAttachment, TextChatMessage};

    use super::*;

    thread_local! {
        static CHILD_RENDER_COUNT: Cell<usize> = const { Cell::new(0) };
    }

    #[component]
    fn RenderProbe() -> Element {
        CHILD_RENDER_COUNT.with(|count| count.set(count.get() + 1));
        rsx! { span { "heavy child" } }
    }

    fn inactive_row_app() -> Element {
        let layout = use_signal(VirtualChatLayout::default);
        rsx! {
            VirtualChatRow {
                row_id: "row".to_owned(),
                active: false,
                estimated_height: 120.0,
                layout,
                RenderProbe {}
            }
        }
    }

    fn active_row_app() -> Element {
        let layout = use_signal(VirtualChatLayout::default);
        rsx! {
            VirtualChatRow {
                row_id: "row".to_owned(),
                active: true,
                estimated_height: 120.0,
                layout,
                RenderProbe {}
            }
        }
    }

    fn windowed_rows_app() -> Element {
        let layout = use_signal(VirtualChatLayout::default);
        let rows = rows(30);
        let rendered = layout.read().rendered_range(&rows);
        rsx! {
            for (index, row) in rows.into_iter().enumerate() {
                VirtualChatRow {
                    key: "{row}",
                    row_id: row,
                    active: rendered.contains(&index),
                    estimated_height: 120.0,
                    layout,
                    RenderProbe {}
                }
            }
        }
    }

    fn rows(count: usize) -> Vec<String> {
        (0..count).map(|index| format!("row-{index}")).collect()
    }

    fn message(id: &str, author: &str, created_at: &str) -> TextChatMessage {
        TextChatMessage {
            id: id.to_owned(),
            server_id: "server".to_owned(),
            room_id: "room".to_owned(),
            author_user_id: author.to_owned(),
            author_nickname: author.to_owned(),
            author_avatar_url: None,
            body: "текст".to_owned(),
            attachments: Vec::new(),
            delivery_status: None,
            created_at: created_at.to_owned(),
        }
    }

    #[test]
    fn small_lists_are_not_virtualized() {
        let layout = VirtualChatLayout::default();
        assert_eq!(
            layout.rendered_range(&rows(SMALL_LIST_ROWS)),
            0..SMALL_LIST_ROWS
        );
    }

    #[test]
    fn initial_window_starts_at_the_bottom_with_overscan() {
        let layout = VirtualChatLayout::default();
        let rows = rows(30);
        assert_eq!(layout.rendered_range(&rows), 24..30);
    }

    #[test]
    fn visible_rows_expand_into_one_contiguous_overscanned_window() {
        let rows = rows(40);
        let mut layout = VirtualChatLayout::default();
        layout.set_visible(&rows[12], true);
        layout.set_visible(&rows[14], true);

        assert_eq!(layout.rendered_range(&rows), 10..17);
    }

    #[test]
    fn overscan_is_clamped_at_both_edges() {
        let rows = rows(20);
        let mut layout = VirtualChatLayout::default();
        layout.set_visible(&rows[0], true);
        assert_eq!(layout.rendered_range(&rows), 0..3);

        layout.set_visible(&rows[0], false);
        layout.set_visible(&rows[19], true);
        assert_eq!(layout.rendered_range(&rows), 17..20);
    }

    #[test]
    fn last_visible_anchor_prevents_a_transient_jump_to_the_bottom() {
        let rows = rows(30);
        let mut layout = VirtualChatLayout::default();
        layout.set_visible(&rows[10], true);
        layout.set_visible(&rows[10], false);

        assert_eq!(layout.rendered_range(&rows), 8..13);
    }

    #[test]
    fn prepending_rows_keeps_the_same_stable_anchor() {
        let original = rows(20);
        let mut layout = VirtualChatLayout::default();
        layout.set_visible(&original[8], true);
        layout.set_visible(&original[8], false);
        let mut prepended = vec!["older-a".to_owned(), "older-b".to_owned()];
        prepended.extend(original);

        assert_eq!(layout.rendered_range(&prepended), 8..13);
        assert_eq!(prepended[10], "row-8");
    }

    #[test]
    fn measured_height_replaces_estimate_and_ignores_noise() {
        let mut layout = VirtualChatLayout::default();
        assert_eq!(layout.placeholder_height("row", 123.0), 123.0);
        assert!(layout.record_height("row", 240.0));
        assert!(!layout.record_height("row", 240.4));
        assert_eq!(layout.placeholder_height("row", 123.0), 240.0);
    }

    #[test]
    fn invalid_measurements_and_estimates_are_sanitized() {
        let mut layout = VirtualChatLayout::default();
        assert!(!layout.record_height("row", 0.0));
        assert!(!layout.record_height("row", f64::NAN));
        assert_eq!(
            layout.placeholder_height("row", f64::INFINITY),
            DEFAULT_ROW_HEIGHT
        );
    }

    #[test]
    fn deleted_rows_are_pruned_from_all_caches() {
        let mut layout = VirtualChatLayout::default();
        layout.set_visible("removed", true);
        layout.record_height("removed", 200.0);
        layout.record_height("saved", 100.0);
        layout.remove_row("removed");

        assert!(layout.visible_rows.is_empty());
        assert_eq!(layout.anchor_row, None);
        assert_eq!(layout.measured_heights.len(), 1);
        assert_eq!(layout.placeholder_height("saved", 50.0), 100.0);
    }

    #[test]
    fn image_estimate_preserves_aspect_ratio_and_bounds() {
        assert_eq!(estimated_image_preview_height(1_600, 900), 293.0);
        assert_eq!(estimated_image_preview_height(900, 1_600), 360.0);
        assert_eq!(estimated_image_preview_height(0, 0), 210.0);
    }

    #[test]
    fn group_estimate_grows_with_text_and_images() {
        let plain = estimated_group_height([4], []);
        let rich = estimated_group_height([4, 200], [293.0, 360.0]);
        assert!(rich > plain + 650.0);
    }

    #[test]
    fn prepared_room_groups_preserve_grouping_dates_and_image_height() {
        let mut first = message("1", "alice", "2025-07-12T08:00:00Z");
        first.attachments.push(TextChatImageAttachment {
            id: "image".to_owned(),
            content_type: "image/png".to_owned(),
            byte_size: 1_024,
            width: 1_600,
            height: 900,
        });
        let messages = [
            first,
            message("2", "alice", "2025-07-12T09:00:00Z"),
            message("3", "bob", "2025-07-12T10:00:00Z"),
            message("4", "bob", "2025-07-13T10:00:00Z"),
        ];

        let groups = prepare_text_chat_groups(&messages);

        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].0, "1");
        assert_eq!(
            groups[0]
                .3
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["1", "2"]
        );
        assert!(groups[0].1.is_some());
        assert!(groups[1].1.is_none());
        assert!(groups[2].1.is_some());
        assert!(groups[0].2 > estimated_group_height([4, 4], []) + 290.0);
    }

    #[test]
    fn inactive_row_does_not_mount_its_heavy_child() {
        CHILD_RENDER_COUNT.with(|count| count.set(0));
        let mut inactive = VirtualDom::new(inactive_row_app);
        inactive.rebuild_in_place();
        assert_eq!(CHILD_RENDER_COUNT.with(Cell::get), 0);

        let mut active = VirtualDom::new(active_row_app);
        active.rebuild_in_place();
        assert_eq!(CHILD_RENDER_COUNT.with(Cell::get), 1);
    }

    #[test]
    fn large_history_mounts_only_the_initial_window_of_heavy_children() {
        CHILD_RENDER_COUNT.with(|count| count.set(0));
        let mut dom = VirtualDom::new(windowed_rows_app);
        dom.rebuild_in_place();

        assert_eq!(CHILD_RENDER_COUNT.with(Cell::get), 6);
    }
}

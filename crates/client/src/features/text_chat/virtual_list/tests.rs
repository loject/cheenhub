//! Сценарии виртуализации списка сообщений текстового чата.

mod restore;

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

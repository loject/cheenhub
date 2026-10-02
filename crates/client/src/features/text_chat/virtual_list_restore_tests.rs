use super::*;

#[test]
fn restored_group_stays_rendered_before_visibility_callbacks_arrive() {
    let rows = (0..30)
        .map(|index| format!("row-{index}"))
        .collect::<Vec<_>>();
    let mut layout = VirtualChatLayout::default();
    layout.set_visible("row-25", true);
    layout.restore_row("row-8".to_owned());
    assert_eq!(layout.rendered_range(&rows), 6..11);
    layout.set_visible("row-25", false);
    assert!(layout.rendered_range(&rows).contains(&8));
    layout.set_visible("row-8", false);
    assert!(layout.rendered_range(&rows).contains(&8));
}

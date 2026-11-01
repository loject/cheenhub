use super::UPDATER_CSS;

#[test]
fn updater_embeds_required_layout_and_progress_styles() {
    assert!(UPDATER_CSS.contains(".grid"));
    assert!(UPDATER_CSS.contains(".application-update-progress-indeterminate"));
}

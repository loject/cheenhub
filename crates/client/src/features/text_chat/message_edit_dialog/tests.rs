use super::can_save_edit;

#[test]
fn empty_text_requires_existing_attachment() {
    assert!(!can_save_edit("   ", "Текст", false));
    assert!(can_save_edit("   ", "Текст", true));
}

#[test]
fn unchanged_or_too_long_text_is_not_saved() {
    assert!(!can_save_edit(" Текст ", "Текст", false));
    assert!(!can_save_edit(&"я".repeat(2001), "Текст", false));
    assert!(can_save_edit("Исправленный текст", "Текст", false));
}

use super::can_save_edit;

#[test]
fn empty_text_requires_existing_attachment() {
    assert!(!can_save_edit("   ", "Текст", false, 2000));
    assert!(can_save_edit("   ", "Текст", true, 2000));
}

#[test]
fn unchanged_or_too_long_text_is_not_saved() {
    assert!(!can_save_edit(" Текст ", "Текст", false, 2000));
    assert!(!can_save_edit(&"я".repeat(2001), "Текст", false, 2000));
    assert!(can_save_edit("Исправленный текст", "Текст", false, 2000));
}

#[test]
fn dm_editor_allows_4000_characters_and_clearing_caption_with_image() {
    assert!(can_save_edit(&"я".repeat(4000), "Текст", false, 4000));
    assert!(!can_save_edit(&"я".repeat(4001), "Текст", false, 4000));
    assert!(can_save_edit("", "Подпись", true, 4000));
    assert!(!can_save_edit("", "Подпись", false, 4000));
}

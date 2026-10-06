use super::{
    can_send_message, format_attachment_size, image_content_type, pending_image_attachment,
};
#[test]
fn rejects_unknown_image_before_upload() {
    assert!(pending_image_attachment(None, b"not an image".to_vec(), 1024).is_err());
}
#[test]
fn formats_compact_attachment_sizes() {
    assert_eq!(format_attachment_size(1_572_864), "1.5 МБ");
}
#[test]
fn rejects_empty_and_oversized_attachments() {
    assert!(pending_image_attachment(None, Vec::new(), 10).is_err());
    assert!(pending_image_attachment(None, vec![0; 11], 10).is_err());
}

#[test]
fn creates_png_data_url_for_supported_thumbnail() {
    let bytes = b"\x89PNG\r\n\x1a\nimage".to_vec();
    let attachment = pending_image_attachment(None, bytes, 1024).unwrap();
    assert_eq!(image_content_type(&attachment.bytes), Some("image/png"));
    assert!(
        attachment
            .preview_data_url
            .starts_with("data:image/png;base64,")
    );
}

#[test]
fn only_enables_send_for_content_when_not_busy() {
    assert!(!can_send_message("   ", false, false));
    assert!(can_send_message("", true, false));
    assert!(can_send_message("текст", false, false));
    assert!(!can_send_message("текст", true, true));
}

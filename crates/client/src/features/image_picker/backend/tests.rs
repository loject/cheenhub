use super::oversized_image_message;

#[test]
fn formats_attachment_limit_in_mebibytes() {
    assert_eq!(
        oversized_image_message(8 * 1024 * 1024),
        "Изображение слишком большое. Максимум — 8 МБ."
    );
}

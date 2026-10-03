use super::source_from_preview;
use crate::features::screen_share::source_preview::MonitorPreview;

#[test]
fn converts_monitor_preview_to_picker_source_with_png_data_url() {
    let source = source_from_preview(MonitorPreview {
        id: "monitor-1".to_owned(),
        display_name: "Основной экран".to_owned(),
        width: 1_920,
        height: 1_080,
        primary: true,
        png_bytes: vec![137, 80, 78, 71],
    });

    assert_eq!(source.id, "monitor-1");
    assert_eq!(source.name, "Основной экран");
    assert_eq!(source.width, 1_920);
    assert_eq!(source.height, 1_080);
    assert!(source.is_primary);
    assert_eq!(source.preview_url, "data:image/png;base64,iVBORw==");
}

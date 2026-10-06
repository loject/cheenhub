use super::{bgra_rows_to_rgba, fit_preview_dimensions};

#[test]
fn fits_wide_source_inside_preview_bounds_without_changing_aspect_ratio() {
    assert_eq!(fit_preview_dimensions(3840, 2160), (640, 360));
}

#[test]
fn converts_bgra_rows_with_padding_to_tightly_packed_rgba() {
    let pixels = [
        10, 20, 30, 40, 50, 60, 70, 80, 200, 201, 202, 203, 90, 100, 110, 120, 130, 140, 150, 160,
        204, 205, 206, 207,
    ];

    let rgba = bgra_rows_to_rgba(&pixels, 2, 2, 12).expect("валидный кадр должен преобразоваться");

    assert_eq!(
        rgba,
        vec![
            30, 20, 10, 40, 70, 60, 50, 80, 110, 100, 90, 120, 150, 140, 130, 160,
        ]
    );
}

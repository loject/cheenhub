use super::*;

#[test]
fn rejects_invalid_source_geometry() {
    assert!(!is_valid_geometry(ChatImageViewerGeometry {
        x: f64::NAN,
        y: 0.0,
        width: 20.0,
        height: 20.0
    }));
    assert!(!is_valid_geometry(ChatImageViewerGeometry {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 20.0
    }));
}

#[test]
fn uniform_flip_maps_top_left_and_opposite_corners_to_source() {
    let source = geometry(10.0, 20.0, 100.0, 50.0);
    let target = geometry(50.0, 70.0, 200.0, 100.0);
    let transform = flip_transform(source, target).expect("valid geometry");

    assert_eq!(transformed_point(target, transform, 0.0, 0.0), (10.0, 20.0));
    assert_eq!(
        transformed_point(target, transform, target.width, target.height),
        (110.0, 70.0)
    );
}

#[test]
fn uniform_flip_keeps_centers_aligned_when_rect_ratios_differ() {
    let source = geometry(20.0, 30.0, 120.0, 60.0);
    let target = geometry(100.0, 100.0, 300.0, 100.0);
    let transform = flip_transform(source, target).expect("valid geometry");

    assert_eq!(transform.scale, 0.4);
    assert_eq!(
        transformed_point(target, transform, target.width / 2.0, target.height / 2.0),
        (80.0, 60.0)
    );
}

#[test]
fn stale_exit_completion_does_not_close_a_reopened_viewer() {
    let mut state = state_fixture("first", 7);
    let closing_generation =
        request_viewer_close(&mut state, 7, "first", Some(geometry(1.0, 1.0, 10.0, 10.0)));
    open_viewer(&mut state, image_fixture("second"), empty_source());
    finish_viewer_close(&mut state, closing_generation.expect("close starts"));
    assert_eq!(
        state.image.expect("viewer remains open").attachment_id,
        "second"
    );
}

#[test]
fn stale_open_request_is_rejected_before_it_can_replace_newer_thumbnail() {
    assert!(!is_current_open_request(8, 7));
    assert!(is_current_open_request(8, 8));
}

#[test]
fn stale_close_request_cannot_close_a_reopened_viewer() {
    let mut state = state_fixture("second", 8);
    assert_eq!(
        request_viewer_close(&mut state, 7, "first", Some(geometry(1.0, 1.0, 10.0, 10.0))),
        None
    );
}

#[test]
fn pointer_pan_preserves_existing_offset() {
    assert_eq!(
        panned_coordinates((120.0, 80.0, 16.0, -24.0), (155.0, 40.0)),
        (51.0, -64.0)
    );
}

fn geometry(x: f64, y: f64, width: f64, height: f64) -> ChatImageViewerGeometry {
    ChatImageViewerGeometry {
        x,
        y,
        width,
        height,
    }
}
fn empty_source() -> ChatImageViewerSource {
    ChatImageViewerSource {
        element: None,
        geometry: None,
    }
}
fn image_fixture(attachment_id: &str) -> ChatImageViewerImage {
    ChatImageViewerImage {
        attachment_id: attachment_id.to_owned(),
        content_type: "image/png".to_owned(),
        data_base64: String::new(),
        width: 1,
        height: 1,
    }
}
fn state_fixture(attachment_id: &str, generation: u64) -> ViewerState {
    ViewerState {
        image: Some(image_fixture(attachment_id)),
        source: empty_source(),
        generation,
        closing_generation: None,
        target_geometry: Some(geometry(0.0, 0.0, 1.0, 1.0)),
        transition: SharedTransition::Enter(None),
    }
}

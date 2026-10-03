use super::{
    ChatImageLoadKey, IMAGE_PREVIEW_MAX_HEIGHT, IMAGE_PREVIEW_MAX_WIDTH, PreviewGeometry,
    image_shell_styles, preview_geometry,
};

#[test]
fn image_preview_geometry_is_bounded() {
    struct Case {
        source: (i32, i32),
        preview: PreviewGeometry,
    }

    let cases = [
        Case {
            source: (0, 0),
            preview: PreviewGeometry {
                width: 280,
                height: 210,
            },
        },
        Case {
            source: (1, 1_000_000_000),
            preview: PreviewGeometry {
                width: 1,
                height: 360,
            },
        },
        Case {
            source: (1_000_000_000, 1),
            preview: PreviewGeometry {
                width: 520,
                height: 1,
            },
        },
        Case {
            source: (520, 360),
            preview: PreviewGeometry {
                width: 520,
                height: 360,
            },
        },
        Case {
            source: (900, 1_600),
            preview: PreviewGeometry {
                width: 203,
                height: 360,
            },
        },
        Case {
            source: (12, 8),
            preview: PreviewGeometry {
                width: 12,
                height: 8,
            },
        },
    ];

    for case in cases {
        let preview = preview_geometry(case.source.0, case.source.1);
        assert_eq!(preview, case.preview);
        assert!(preview.width > 0 && preview.height > 0);
        assert!(f64::from(preview.width) <= IMAGE_PREVIEW_MAX_WIDTH);
        assert!(f64::from(preview.height) <= IMAGE_PREVIEW_MAX_HEIGHT);
    }
}

#[test]
fn shell_styles_keep_a_definite_width_outside_intrinsic_percentage_sizing() {
    let styles = image_shell_styles(preview_geometry(1_600, 900));

    assert_eq!(styles.wrapper, "width: 520px; max-width: 100%;");
    assert!(!styles.wrapper.contains("min(100%"));
    assert_eq!(styles.surface, "aspect-ratio: 520 / 293;");
}

#[test]
fn image_load_key_includes_server_room_and_attachment() {
    let server = "00000000-0000-0000-0000-000000000001";
    let room = "00000000-0000-0000-0000-000000000002";
    let attachment = "00000000-0000-0000-0000-000000000003";

    let key = ChatImageLoadKey::parse(server, room, attachment).expect("valid UUID key");
    let another_room =
        ChatImageLoadKey::parse(server, "00000000-0000-0000-0000-000000000004", attachment)
            .expect("valid UUID key");
    let another_server =
        ChatImageLoadKey::parse("00000000-0000-0000-0000-000000000005", room, attachment)
            .expect("valid UUID key");
    let another_attachment =
        ChatImageLoadKey::parse(server, room, "00000000-0000-0000-0000-000000000006")
            .expect("valid UUID key");

    assert_ne!(key, another_room);
    assert_ne!(key, another_server);
    assert_ne!(key, another_attachment);
    assert_ne!(
        ChatImageLoadKey::render_key(server, room, attachment),
        ChatImageLoadKey::render_key(server, room, "00000000-0000-0000-0000-000000000006")
    );
    assert_ne!(
        ChatImageLoadKey::render_key(server, room, attachment),
        ChatImageLoadKey::render_key("00000000-0000-0000-0000-000000000005", room, attachment)
    );
    assert_ne!(
        ChatImageLoadKey::render_key(server, room, attachment),
        ChatImageLoadKey::render_key(server, "00000000-0000-0000-0000-000000000004", attachment)
    );
}

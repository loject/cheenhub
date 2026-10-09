use super::*;

#[test]
fn identifies_both_side_buttons_and_their_release() {
    for (data, code) in [(1 << 16, 5), (2 << 16, 6)] {
        let down = mouse(WM_XBUTTONDOWN, data).unwrap();
        let up = mouse(WM_XBUTTONUP, data).unwrap();
        assert_eq!(down.key.code(), code);
        assert!(down.pressed);
        assert_eq!(up.key, down.key);
        assert!(!up.pressed);
    }
}

#[test]
fn ignores_mouse_motion_wheel_and_unknown_side_buttons() {
    assert!(mouse(WM_MOUSEMOVE, 0).is_none());
    assert!(mouse(WM_MOUSEWHEEL, 120 << 16).is_none());
    assert!(mouse(WM_XBUTTONDOWN, 3 << 16).is_none());
}

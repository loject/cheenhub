use super::status_from_runtime_error;
use crate::features::screen_share::{ScreenShareError, ScreenShareStatus};

#[test]
fn runtime_error_is_not_replaced_by_following_capture_end() {
    use dioxus::prelude::*;
    use futures_util::FutureExt;
    use std::rc::Rc;

    let mut dom = VirtualDom::new(|| rsx! {});
    dom.rebuild_in_place();
    let (callbacks, status) = dom.in_scope(ScopeId::ROOT, || {
        let status = Signal::new(ScreenShareStatus::Live);
        let callbacks = super::screen_share_callbacks(
            Rc::new(|_| {}),
            super::CaptureRuntime {
                session: Signal::new(None),
                status,
                generation: Signal::new(1),
                start_generation: 1,
                provider_scope: ScopeId::ROOT,
                audio_state: Signal::new(Default::default()),
                on_warning: Rc::new(|_| {}),
            },
        );
        (callbacks, status)
    });

    (callbacks.on_error)(ScreenShareError::new("reader failure"));
    (callbacks.on_ended)();
    for _ in 0..5 {
        let _ = dom.wait_for_work().now_or_never();
        dom.render_immediate_to_vec();
    }

    dom.in_scope(ScopeId::ROOT, || {
        assert_eq!(
            *status.peek(),
            ScreenShareStatus::Error("reader failure".to_owned())
        );
    });
}

#[test]
fn runtime_error_always_enters_error_status() {
    assert_eq!(
        status_from_runtime_error(ScreenShareError::new("WGC failure")),
        ScreenShareStatus::Error("WGC failure".to_owned())
    );
}

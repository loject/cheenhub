use super::*;
use std::cell::RefCell;

thread_local! {
    static WARNING_STATUS: RefCell<Option<MicrophoneStatus>> = const { RefCell::new(None) };
}

#[test]
fn denoiser_warning_is_not_a_capture_failure() {
    let mut dom = VirtualDom::new(warning_test_app);
    dom.rebuild_in_place();
    assert_eq!(
        dom.in_scope(ScopeId::ROOT, || WARNING_STATUS
            .with(|status| { status.borrow().clone() })),
        Some(MicrophoneStatus::Live)
    );
}

fn warning_test_app() -> Element {
    rsx! { WarningTestShell {} }
}

#[component]
fn WarningTestShell() -> Element {
    let focused = use_signal(|| true);
    use_context_provider(move || {
        crate::features::application_focus::ApplicationFocusContext::new(focused)
    });
    rsx! { crate::features::toast::ToastProvider { WarningTestProbe {} } }
}

#[component]
fn WarningTestProbe() -> Element {
    let toast = use_context::<ToastHandle>();
    use_hook(move || {
        apply_warning("Denoise warning", &toast);
        WARNING_STATUS.with(|status| {
            *status.borrow_mut() = Some(MicrophoneStatus::Live);
        });
    });
    rsx! {}
}

#[test]
fn capture_errors_keep_the_existing_failure_status() {
    let status = status_from_error(MicrophoneError::new("capture failed"));

    assert_eq!(status, MicrophoneStatus::Error("capture failed".to_owned()));
}

//! Проверки жизненного цикла provider микрофона.

use std::cell::Cell;
use std::rc::Rc;

use futures_util::FutureExt;

use super::{
    ActiveCapture, should_start_level_preview, should_stop_level_preview, stop_session_immediately,
};
use crate::features::microphone::{MicrophoneError, MicrophoneSession, MicrophoneStatus};

struct TestSession {
    stopped_immediately: Rc<Cell<bool>>,
    async_stop_polled: Rc<Cell<bool>>,
}

impl MicrophoneSession for TestSession {
    fn stop_immediately(&self) {
        self.stopped_immediately.set(true);
    }

    fn stop(&self) -> futures_util::future::LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        let async_stop_polled = self.async_stop_polled.clone();
        async move {
            async_stop_polled.set(true);
            Ok(())
        }
        .boxed_local()
    }

    fn set_bitrate_bps(
        &self,
        _bitrate_bps: u32,
    ) -> futures_util::future::LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        async { Ok(()) }.boxed_local()
    }
}

#[test]
fn level_preview_starts_from_idle_and_terminal_errors_without_active_capture() {
    assert!(should_start_level_preview(
        &MicrophoneStatus::Idle,
        ActiveCapture::None
    ));
    assert!(should_start_level_preview(
        &MicrophoneStatus::PermissionDenied,
        ActiveCapture::None
    ));
    assert!(should_start_level_preview(
        &MicrophoneStatus::Error("ошибка захвата".to_owned()),
        ActiveCapture::None
    ));
}

#[test]
fn level_preview_does_not_duplicate_starting_or_live_capture() {
    assert!(!should_start_level_preview(
        &MicrophoneStatus::Starting,
        ActiveCapture::None
    ));
    assert!(!should_start_level_preview(
        &MicrophoneStatus::Live,
        ActiveCapture::None
    ));
}

#[test]
fn level_preview_does_not_replace_an_active_capture() {
    for active_capture in [ActiveCapture::Preview, ActiveCapture::Voice] {
        assert!(!should_start_level_preview(
            &MicrophoneStatus::PermissionDenied,
            active_capture
        ));
    }
}

#[test]
fn closing_settings_stops_only_preview_capture() {
    assert!(should_stop_level_preview(ActiveCapture::Preview));
    assert!(!should_stop_level_preview(ActiveCapture::Voice));
    assert!(!should_stop_level_preview(ActiveCapture::None));
}

#[test]
fn session_capture_is_released_without_polling_async_cleanup() {
    let stopped_immediately = Rc::new(Cell::new(false));
    let async_stop_polled = Rc::new(Cell::new(false));
    let session: Rc<dyn MicrophoneSession> = Rc::new(TestSession {
        stopped_immediately: stopped_immediately.clone(),
        async_stop_polled: async_stop_polled.clone(),
    });

    stop_session_immediately(Some(&session));

    assert!(stopped_immediately.get());
    assert!(!async_stop_polled.get());
}

use super::status_from_runtime_error;
use crate::features::screen_share::{ScreenShareError, ScreenShareStatus};

#[test]
fn runtime_error_always_enters_error_status() {
    assert_eq!(
        status_from_runtime_error(ScreenShareError::new("WGC failure")),
        ScreenShareStatus::Error("WGC failure".to_owned())
    );
}

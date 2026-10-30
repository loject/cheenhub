use super::*;
use std::path::Path;

#[cfg(not(target_os = "windows"))]
#[test]
fn unix_log_directory_uses_absolute_xdg_state_then_home_fallback() {
    let state = Path::new("/tmp/cheenhub-state-test");
    let home = Path::new("/tmp/cheenhub-home-test");
    assert_eq!(
        unix_log_directory(Some(state), Some(home)),
        Some(state.join("cheenhub/logs"))
    );
    assert_eq!(
        unix_log_directory(Some(Path::new("relative")), Some(home)),
        Some(home.join(".local/state/cheenhub/logs"))
    );
    assert_eq!(unix_log_directory(None, None), None);
}

#[cfg(target_os = "windows")]
#[test]
fn windows_log_directory_uses_local_app_data() {
    let app_data = Path::new(r"C:\Users\test\AppData\Local");
    assert_eq!(
        windows_log_directory(Some(app_data)),
        Some(app_data.join("CheenHub").join("logs"))
    );
    assert_eq!(windows_log_directory(None), None);
}

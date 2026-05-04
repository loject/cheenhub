//! Placeholder behavior for landing page actions.

#[cfg(target_arch = "wasm32")]
pub(crate) fn show_todo_alert() {
    if let Some(window) = web_sys::window() {
        let _ = window.alert_with_message("Эта возможность скоро появится.");
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn show_todo_alert() {}

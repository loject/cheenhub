//! Выбор реализации системного входа через Google для текущей платформы.

#[cfg(target_os = "android")]
#[path = "android.rs"]
mod platform;

#[cfg(not(target_os = "android"))]
#[path = "unsupported.rs"]
mod platform;

pub(super) use platform::{is_supported, request_google_id_token};

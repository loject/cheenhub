//! Выбор native-реализации WebTransport TLS для текущей платформы.

#[cfg(target_os = "android")]
#[path = "android.rs"]
mod target;

#[cfg(not(target_os = "android"))]
#[path = "desktop.rs"]
mod target;

pub(in crate::features::realtime) use target::connect;

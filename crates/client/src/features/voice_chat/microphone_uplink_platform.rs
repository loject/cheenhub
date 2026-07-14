//! Выбор платформенной реализации microphone uplink.

#[cfg(feature = "web")]
#[path = "microphone_uplink/web.rs"]
mod implementation;

#[cfg(not(feature = "web"))]
#[path = "microphone_uplink/native.rs"]
mod implementation;

pub(crate) use implementation::{restart, toggle};

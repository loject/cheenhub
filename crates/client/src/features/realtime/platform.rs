//! Выбор реализации WebTransport-подключения для текущей платформы.

#[cfg(not(target_arch = "wasm32"))]
#[path = "platform/native.rs"]
mod implementation;

#[cfg(target_arch = "wasm32")]
#[path = "platform/web.rs"]
mod implementation;

pub(in crate::features::realtime) use implementation::connect;

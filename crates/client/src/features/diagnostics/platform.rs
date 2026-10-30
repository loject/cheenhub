//! Выбор платформенной реализации локальной диагностики.

#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
#[path = "platform/native.rs"]
mod implementation;

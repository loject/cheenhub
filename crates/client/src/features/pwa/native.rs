//! Выбор платформенной реализации PWA-интеграции.

#[cfg(target_family = "wasm")]
pub(crate) use super::web::PwaVersionBridge;

#[cfg(not(target_family = "wasm"))]
pub(crate) use super::unsupported::PwaVersionBridge;

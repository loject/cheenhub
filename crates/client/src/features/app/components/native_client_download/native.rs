//! Выбор платформенной реализации скачивания native-клиента.

#[cfg(not(target_family = "wasm"))]
pub(crate) use super::unsupported::NativeClientDownload;
#[cfg(target_family = "wasm")]
pub(crate) use super::web::NativeClientDownload;

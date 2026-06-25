#![warn(missing_docs)]
//! Библиотечная точка входа для wasm-воркеров web-клиента.

#[cfg(feature = "microphone_worker")]
pub mod workers;

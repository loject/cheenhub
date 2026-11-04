#![warn(missing_docs)]
//! Библиотечная точка входа для wasm-воркеров web-клиента.

/// Безопасный разбор Markdown в дерево для отображения сообщений.
#[path = "features/markdown/mod.rs"]
pub mod markdown;

#[path = "features/microphone/core.rs"]
mod microphone_core;

#[path = "features/microphone/denoise/dsp/mod.rs"]
mod microphone_denoise;

pub mod workers;

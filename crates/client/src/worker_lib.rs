#![warn(missing_docs)]
//! Библиотечная точка входа для wasm-воркеров web-клиента.

#[path = "features/microphone/core.rs"]
mod microphone_core;

#[path = "features/microphone/denoise/dsp/mod.rs"]
mod microphone_denoise;

pub mod workers;

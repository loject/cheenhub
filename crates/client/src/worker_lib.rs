#![warn(missing_docs)]
//! Библиотечная точка входа для wasm-воркеров web-клиента.

#[path = "features/microphone/core.rs"]
mod microphone_core;

pub mod workers;

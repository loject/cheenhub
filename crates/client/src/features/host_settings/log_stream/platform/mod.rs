//! Выбор платформенной реализации потока логов.

mod native;
mod unsupported;
mod web;

pub(super) use native::run;

//! PWA-интеграция web-клиента.

mod native;
mod unsupported;
mod web;

pub(crate) use native::PwaVersionBridge;

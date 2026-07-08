//! Web-only entry point for the native client download control.

mod native;
mod unsupported;
mod web;

pub(crate) use native::NativeClientDownload;

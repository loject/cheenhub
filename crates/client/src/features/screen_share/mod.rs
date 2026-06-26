//! Функция захвата и кодирования демонстрации экрана.

mod backend;
mod browser;
mod browser_capture;
mod browser_errors;
mod native;
mod provider;
mod provider_context;
mod provider_runtime;
mod unsupported;

#[allow(unused_imports)]
pub(crate) use backend::{
    EncodedScreenShareFrame, ScreenShareBackend, ScreenShareCallbacks, ScreenShareCodec,
    ScreenShareConfig, ScreenShareEndedCallback, ScreenShareError, ScreenShareFrameCallback,
    ScreenShareSession, ScreenShareStatus,
};
pub(crate) use provider::ScreenShareHandle;
pub(crate) use provider_context::ScreenShareProvider;

//! Функция захвата и кодирования демонстрации экрана.

mod backend;
#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(target_arch = "wasm32")]
mod browser_capture;
#[cfg(target_arch = "wasm32")]
mod browser_errors;
mod provider;
mod provider_context;
mod provider_runtime;
#[cfg(not(target_arch = "wasm32"))]
mod unsupported;

#[allow(unused_imports)]
pub(crate) use backend::{
    EncodedScreenShareFrame, ScreenShareBackend, ScreenShareCallbacks, ScreenShareCodec,
    ScreenShareConfig, ScreenShareEndedCallback, ScreenShareError, ScreenShareFrameCallback,
    ScreenShareSession, ScreenShareStatus,
};
pub(crate) use provider::ScreenShareHandle;
pub(crate) use provider_context::ScreenShareProvider;

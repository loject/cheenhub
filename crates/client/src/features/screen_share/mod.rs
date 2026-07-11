//! Функция захвата и кодирования демонстрации экрана.

#[cfg(target_os = "android")]
mod android;
mod backend;
mod browser;
mod browser_capture;
mod browser_errors;
mod native;
mod provider;
mod provider_context;
mod provider_runtime;
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
mod unsupported;

#[allow(unused_imports)]
pub(crate) use backend::{
    EncodedScreenShareFrame, ScreenShareBackend, ScreenShareCallbacks, ScreenShareCodec,
    ScreenShareConfig, ScreenShareEndedCallback, ScreenShareError, ScreenShareFrameCallback,
    ScreenShareSession, ScreenShareStatus,
};
pub(crate) use provider::ScreenShareHandle;
pub(crate) use provider_context::ScreenShareProvider;

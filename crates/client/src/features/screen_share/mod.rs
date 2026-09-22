//! Функция захвата и кодирования демонстрации экрана.

#[cfg(target_os = "android")]
mod android;
mod backend;
mod browser;
mod browser_capture;
mod browser_errors;
mod native;
mod picker_flow;
mod picker_host;
mod provider;
mod provider_context;
mod provider_runtime;
mod source_picker;
mod source_preview;
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
mod unsupported;

#[allow(unused_imports)]
pub(crate) use backend::{
    EncodedScreenShareFrame, ScreenShareBackend, ScreenShareCallbacks, ScreenShareCaptureSource,
    ScreenShareCodec, ScreenShareConfig, ScreenShareEndedCallback, ScreenShareError,
    ScreenShareErrorCallback, ScreenShareFrameCallback, ScreenShareSession,
    ScreenShareStartRequest, ScreenShareStatus, ScreenShareTargetQuality,
};
pub(crate) use provider::ScreenShareHandle;
pub(crate) use provider_context::ScreenShareProvider;

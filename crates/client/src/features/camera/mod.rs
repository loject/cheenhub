//! Функция захвата и кодирования камеры.

#[cfg(target_os = "android")]
mod android;
mod backend;
mod browser_capture;
mod browser_errors;
mod native;
mod provider;
mod provider_context;
mod provider_runtime;
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
mod unsupported;
mod web;

#[allow(unused_imports)]
pub(crate) use backend::{
    CameraBackend, CameraCallbacks, CameraCodec, CameraConfig, CameraEndedCallback, CameraError,
    CameraFrameCallback, CameraSession, CameraStatus, EncodedCameraFrame,
};
pub(crate) use provider::CameraHandle;
pub(crate) use provider_context::CameraProvider;

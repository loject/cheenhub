//! Функция захвата и кодирования камеры.

mod backend;
#[cfg(target_arch = "wasm32")]
mod browser_bindings;
#[cfg(target_arch = "wasm32")]
mod browser_capture;
#[cfg(target_arch = "wasm32")]
mod browser_errors;
mod provider;
mod provider_context;
mod provider_runtime;
#[cfg(not(target_arch = "wasm32"))]
mod unsupported;
#[cfg(target_arch = "wasm32")]
mod web;

#[allow(unused_imports)]
pub(crate) use backend::{
    CameraBackend, CameraCallbacks, CameraCodec, CameraConfig, CameraEndedCallback, CameraError,
    CameraFrameCallback, CameraSession, CameraStatus, EncodedCameraFrame,
};
pub(crate) use provider::CameraHandle;
pub(crate) use provider_context::CameraProvider;

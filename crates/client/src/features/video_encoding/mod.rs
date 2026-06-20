//! Общая функция кодирования видео.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

mod backend;
#[cfg(not(target_arch = "wasm32"))]
mod unsupported;
#[cfg(target_arch = "wasm32")]
mod web;

pub(crate) use backend::{
    EncodedVideoFrame, VideoCodec, VideoEncoderConfig, VideoEncodingAcceleratorKind,
    VideoEncodingError, VideoEncodingManager, VideoFrameEncoder,
};
#[allow(unused_imports)]
pub(crate) use backend::{
    EncodedVideoFrameCallback, VideoEncoderDescriptor, VideoEncodingAccelerator,
};
#[cfg(not(target_arch = "wasm32"))]
#[allow(unused_imports)]
pub(crate) use unsupported::UnsupportedVideoEncodingManager;
#[cfg(target_arch = "wasm32")]
pub(crate) use web::{
    BrowserVideoEncoder, BrowserVideoEncoderHandle, BrowserVideoEncodingManager,
    BrowserVideoFrameReader,
};

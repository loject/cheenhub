//! Общая функция кодирования видео.

mod backend;
mod unsupported;
mod web;

pub(crate) use backend::{
    EncodedVideoFrame, VideoCodec, VideoEncoderConfig, VideoEncodingAcceleratorKind,
    VideoEncodingError, VideoEncodingManager, VideoFrameEncoder,
};
#[allow(unused_imports)]
pub(crate) use backend::{
    EncodedVideoFrameCallback, VideoEncoderDescriptor, VideoEncodingAccelerator,
};
#[allow(unused_imports)]
pub(crate) use unsupported::UnsupportedVideoEncodingManager;
pub(crate) use web::{
    BrowserVideoEncoder, BrowserVideoEncoderHandle, BrowserVideoEncodingManager,
    BrowserVideoFrameReader,
};

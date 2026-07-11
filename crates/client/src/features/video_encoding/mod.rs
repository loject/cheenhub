//! Общая функция кодирования видео.

#[cfg(target_os = "android")]
mod android;
mod backend;
mod unsupported;
mod web;

#[cfg(target_os = "android")]
pub(crate) use android::{
    AndroidSurfaceVideoEncoder, AndroidVideoCaptureSession, AndroidVideoEncodingManager,
    android_video_capture_bridge,
};
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

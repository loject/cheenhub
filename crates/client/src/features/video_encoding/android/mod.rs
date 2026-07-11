//! Android-реализация захвата и кодирования видео.

mod capture;
mod encoder;

pub(crate) use capture::{AndroidVideoCaptureSession, android_video_capture_bridge};
pub(crate) use encoder::{AndroidSurfaceVideoEncoder, AndroidVideoEncodingManager};

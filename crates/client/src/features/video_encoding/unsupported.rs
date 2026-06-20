//! Заглушка кодирования видео для неподдерживаемых платформ.

use futures_util::FutureExt;

use super::backend::{
    EncodedVideoFrameCallback, VideoEncoderConfig, VideoEncoderDescriptor,
    VideoEncodingAcceleratorKind, VideoEncodingError, VideoEncodingManager, VideoFrameEncoder,
};

/// Менеджер кодировщиков видео для платформ без реализации.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct UnsupportedVideoEncodingManager;

impl VideoEncodingManager for UnsupportedVideoEncodingManager {
    type Encoder = UnsupportedVideoEncoder;
    type InputFrame = ();

    fn available_accelerators(
        &self,
        _config: VideoEncoderConfig,
    ) -> futures_util::future::LocalBoxFuture<
        'static,
        Result<Vec<VideoEncoderDescriptor>, VideoEncodingError>,
    > {
        async { Ok(Vec::new()) }.boxed_local()
    }

    fn create_encoder(
        &self,
        kind: VideoEncodingAcceleratorKind,
        _config: VideoEncoderConfig,
        _on_frame: EncodedVideoFrameCallback,
    ) -> futures_util::future::LocalBoxFuture<'static, Result<Self::Encoder, VideoEncodingError>>
    {
        async move {
            Err(VideoEncodingError::unsupported(format!(
                "Кодировщик {kind:?} недоступен на текущей платформе."
            )))
        }
        .boxed_local()
    }
}

/// Заглушка активного кодировщика видео.
#[derive(Debug, Clone, Copy)]
pub(crate) struct UnsupportedVideoEncoder;

impl VideoFrameEncoder for UnsupportedVideoEncoder {
    type InputFrame = ();

    fn encode(
        &self,
        _frame: &Self::InputFrame,
        _key_frame: bool,
    ) -> Result<(), VideoEncodingError> {
        Err(VideoEncodingError::unsupported(
            "Кодирование видео недоступно на текущей платформе.",
        ))
    }

    fn close(&self) -> Result<(), VideoEncodingError> {
        Ok(())
    }
}

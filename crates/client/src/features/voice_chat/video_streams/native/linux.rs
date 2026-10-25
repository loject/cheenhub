//! Linux renderer входящего VP9 через live WebM и штатный HTML video.

mod stream;
mod vp9;
mod webm;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use dioxus::logger::tracing::{debug, info};
use futures_util::future::LocalBoxFuture;

use super::super::{
    ParticipantVideoFrame,
    backend::{
        ParticipantVideoBackend, ParticipantVideoDimensionsCallback, ParticipantVideoRenderError,
        ParticipantVideoRenderer,
    },
};

/// Linux backend, передающий исходный VP9 штатному WebKitGTK/GStreamer плееру.
pub(super) struct LinuxParticipantVideoBackend;

impl ParticipantVideoBackend for LinuxParticipantVideoBackend {
    fn create_renderer(
        &self,
        _target_id: String,
        user_id: String,
        source_label: &'static str,
        on_dimensions: ParticipantVideoDimensionsCallback,
    ) -> Result<Rc<dyn ParticipantVideoRenderer>, ParticipantVideoRenderError> {
        let stream = stream::LocalVideoStream::new().map_err(|error| {
            ParticipantVideoRenderError::new(format!(
                "Не удалось открыть локальный видеопоток: {error}"
            ))
        })?;
        info!(sender_user_id = %user_id, source = source_label, "created Linux WebM participant video renderer");
        Ok(Rc::new(LinuxParticipantVideoRenderer {
            stream: RefCell::new(stream),
            muxer: RefCell::new(webm::WebmMuxer::default()),
            dimensions: Cell::new(None),
            on_dimensions,
            user_id,
            source_label,
            closed: Cell::new(false),
        }))
    }
}

struct LinuxParticipantVideoRenderer {
    stream: RefCell<stream::LocalVideoStream>,
    muxer: RefCell<webm::WebmMuxer>,
    dimensions: Cell<Option<(u32, u32)>>,
    on_dimensions: ParticipantVideoDimensionsCallback,
    user_id: String,
    source_label: &'static str,
    closed: Cell<bool>,
}

impl ParticipantVideoRenderer for LinuxParticipantVideoRenderer {
    fn playback_url(&self) -> Option<String> {
        Some(self.stream.borrow().url().to_owned())
    }

    fn decode(
        &self,
        frame: ParticipantVideoFrame,
    ) -> LocalBoxFuture<'_, Result<(), ParticipantVideoRenderError>> {
        Box::pin(async move {
            if self.closed.get() {
                return Err(ParticipantVideoRenderError::new("Плеер видео закрыт"));
            }
            let packet = self
                .muxer
                .borrow_mut()
                .push(
                    &frame.bytes,
                    frame.timestamp_us,
                    frame.duration_us,
                    frame.key_frame,
                )
                .map_err(ParticipantVideoRenderError::new)?;
            let Some(packet) = packet else {
                return Ok(());
            };
            self.stream
                .borrow()
                .send(packet.bytes)
                .map_err(ParticipantVideoRenderError::new)?;
            if let Some(dimensions) = packet.dimensions {
                let size = (dimensions.display_width, dimensions.display_height);
                if self.dimensions.replace(Some(size)) != Some(size) {
                    info!(sender_user_id = %self.user_id, source = self.source_label,
                        width = size.0, height = size.1, "Linux participant video dimensions changed");
                    (self.on_dimensions)(size.0, size.1);
                }
            }
            Ok(())
        })
    }

    fn close(&self) {
        if self.closed.replace(true) {
            return;
        }
        self.stream.borrow_mut().close();
        debug!(sender_user_id = %self.user_id, source = self.source_label, "closed Linux participant video renderer");
    }
}

impl Drop for LinuxParticipantVideoRenderer {
    fn drop(&mut self) {
        self.close();
    }
}

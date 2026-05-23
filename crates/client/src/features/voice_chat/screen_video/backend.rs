//! Platform contract for rendering inbound screen sharing video.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::features::voice_chat::realtime::InboundScreenFrame;

/// Platform-independent screen sharing video renderer factory.
pub(crate) trait ScreenVideoBackend {
    /// Creates a renderer attached to a Dioxus-owned target element.
    fn create_renderer(
        &self,
        target_id: String,
        user_id: String,
    ) -> Result<Rc<dyn ScreenVideoRenderer>, ScreenVideoRenderError>;
}

/// Platform renderer for one participant's screen sharing stream.
pub(crate) trait ScreenVideoRenderer {
    /// Decodes and renders one inbound encoded VP9 frame.
    fn decode(&self, frame: &InboundScreenFrame) -> Result<(), ScreenVideoRenderError>;

    /// Releases platform decoder and rendering resources.
    fn close(&self);
}

/// User-visible rendering setup or decode failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScreenVideoRenderError {
    message: String,
}

impl ScreenVideoRenderError {
    /// Builds a rendering error.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for ScreenVideoRenderError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ScreenVideoRenderError {}

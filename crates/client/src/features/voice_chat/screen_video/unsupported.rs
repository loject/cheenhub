//! Fallback screen sharing renderer for unsupported platforms.

use std::rc::Rc;

use super::backend::{ScreenVideoBackend, ScreenVideoRenderError, ScreenVideoRenderer};

/// Screen sharing video backend used when no platform renderer is available.
#[derive(Debug, Default)]
pub(crate) struct UnavailableScreenVideoBackend;

impl ScreenVideoBackend for UnavailableScreenVideoBackend {
    fn create_renderer(
        &self,
        _target_id: String,
        _user_id: String,
    ) -> Result<Rc<dyn ScreenVideoRenderer>, ScreenVideoRenderError> {
        Err(ScreenVideoRenderError::new(
            "Отображение демонстрации экрана недоступно на этой платформе.",
        ))
    }
}

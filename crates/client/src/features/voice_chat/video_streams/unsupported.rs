//! Запасной renderer видео участника для неподдерживаемых платформ.

use std::rc::Rc;

use super::backend::{
    ParticipantVideoBackend, ParticipantVideoRenderError, ParticipantVideoRenderer,
};

/// Backend видео участника, используемый без доступного платформенного renderer'а.
#[derive(Debug, Default)]
pub(crate) struct UnavailableParticipantVideoBackend;

impl ParticipantVideoBackend for UnavailableParticipantVideoBackend {
    fn create_renderer(
        &self,
        _target_id: String,
        _user_id: String,
        source_label: &'static str,
    ) -> Result<Rc<dyn ParticipantVideoRenderer>, ParticipantVideoRenderError> {
        Err(ParticipantVideoRenderError::new(format!(
            "Отображение {source_label} недоступно на этой платформе."
        )))
    }
}

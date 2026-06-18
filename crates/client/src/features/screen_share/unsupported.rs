//! Неподдерживаемый backend демонстрации экрана.

use std::rc::Rc;

use futures_util::FutureExt;
use futures_util::future::LocalBoxFuture;

use super::backend::{
    ScreenShareBackend, ScreenShareCallbacks, ScreenShareConfig, ScreenShareError,
    ScreenShareSession,
};

/// Backend демонстрации экрана для платформ без поддержки захвата.
pub(crate) struct UnavailableScreenShareBackend;

impl ScreenShareBackend for UnavailableScreenShareBackend {
    fn start(
        &self,
        _config: ScreenShareConfig,
        callbacks: ScreenShareCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn ScreenShareSession>, ScreenShareError>> {
        let ScreenShareCallbacks {
            on_frame: _on_frame,
            on_ended: _on_ended,
        } = callbacks;

        async move {
            Err(ScreenShareError::new("Демонстрация экрана недоступна."))
        }
        .boxed_local()
    }
}

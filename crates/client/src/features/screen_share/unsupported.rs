//! Unsupported screen sharing backend.

use std::rc::Rc;

use futures_util::FutureExt;
use futures_util::future::LocalBoxFuture;

use super::backend::{
    ScreenShareBackend, ScreenShareCallbacks, ScreenShareConfig, ScreenShareError,
    ScreenShareSession,
};

/// Screen sharing backend for platforms without capture support.
pub(crate) struct UnavailableScreenShareBackend;

impl ScreenShareBackend for UnavailableScreenShareBackend {
    fn start(
        &self,
        _config: ScreenShareConfig,
        _callbacks: ScreenShareCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn ScreenShareSession>, ScreenShareError>> {
        async move {
            Err(ScreenShareError::new("Демонстрация экрана недоступна."))
        }
        .boxed_local()
    }
}

//! Native-заглушка backend'а микрофона.
#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports))]

use std::rc::Rc;

use dioxus::prelude::warn;
use futures_util::FutureExt;
use futures_util::future::LocalBoxFuture;

use super::backend::{
    MicrophoneBackend, MicrophoneCallbacks, MicrophoneConfig, MicrophoneError, MicrophoneSession,
};

/// Backend микрофона для платформ без поддержки browser capture.
#[derive(Debug, Default)]
pub(crate) struct UnavailableMicrophoneBackend;

impl MicrophoneBackend for UnavailableMicrophoneBackend {
    fn start(
        &self,
        _config: MicrophoneConfig,
        _callbacks: MicrophoneCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn MicrophoneSession>, MicrophoneError>> {
        async move {
            warn!("native microphone capture backend is unavailable");
            Err(MicrophoneError::new(
                "Захват микрофона недоступен в native-клиенте.",
            ))
        }
        .boxed_local()
    }
}

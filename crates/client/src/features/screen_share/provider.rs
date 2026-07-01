//! Контекстный хэндл демонстрации экрана.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::features::toast::ToastHandle;

use super::backend::{
    ScreenShareBackend, ScreenShareConfig, ScreenShareFrameCallback, ScreenShareSession,
    ScreenShareStatus,
};
use super::provider_runtime::{next_generation, screen_share_callbacks, status_from_error};

/// Контекстный хэндл, используемый функциями, которым нужен захват экрана.
#[derive(Clone)]
pub(crate) struct ScreenShareHandle {
    pub(super) status: Signal<ScreenShareStatus>,
    pub(super) session: Signal<Option<Rc<dyn ScreenShareSession>>>,
    pub(super) generation: Signal<u64>,
    pub(super) backend: Rc<dyn ScreenShareBackend>,
    pub(super) toast: ToastHandle,
}

impl ScreenShareHandle {
    /// Starts screen sharing capture with the default configuration.
    pub(crate) fn start(&self, on_frame: ScreenShareFrameCallback) {
        if matches!(
            self.status(),
            ScreenShareStatus::Starting | ScreenShareStatus::Live
        ) {
            return;
        }

        let backend = self.backend.clone();
        let mut session = self.session;
        let mut status = self.status;
        let mut generation = self.generation;
        let toast = self.toast;
        let start_generation = next_generation(&mut generation);
        status.set(ScreenShareStatus::Starting);

        info!("starting screen sharing capture");
        spawn(async move {
            let callbacks = screen_share_callbacks(on_frame.clone(), session, status, generation);
            match backend.start(ScreenShareConfig::default(), callbacks).await {
                Ok(next_session) => {
                    if generation() != start_generation {
                        if let Err(error) = next_session.stop().await {
                            warn!(
                                %error,
                                "failed to stop stale screen sharing capture after start"
                            );
                        }
                        return;
                    }
                    session.set(Some(next_session));
                    status.set(ScreenShareStatus::Live);
                    info!("screen sharing capture started");
                }
                Err(error) => {
                    if generation() != start_generation {
                        return;
                    }
                    let next_status = status_from_error(error.clone());
                    warn!(%error, status = ?next_status, "failed to start screen sharing capture");
                    if error.is_unsupported_browser() {
                        toast.warning(error.to_string());
                    }
                    session.set(None);
                    status.set(next_status);
                }
            }
        });
    }

    /// Stops the active screen sharing session.
    pub(crate) fn stop(&self) {
        let mut generation = self.generation;
        let stop_generation = next_generation(&mut generation);
        let Some(active_session) = self.session.peek().clone() else {
            let mut status = self.status;
            status.set(ScreenShareStatus::Idle);
            return;
        };

        let mut session = self.session;
        let mut status = self.status;
        info!("stopping screen sharing capture");
        spawn(async move {
            if let Err(error) = active_session.stop().await {
                warn!(%error, "failed to stop screen sharing capture cleanly");
            }
            if generation() != stop_generation {
                return;
            }
            session.set(None);
            status.set(ScreenShareStatus::Idle);
            info!("screen sharing capture stopped");
        });
    }

    /// Toggles screen sharing capture.
    pub(crate) fn toggle(&self, on_frame: ScreenShareFrameCallback) {
        if matches!(
            self.status(),
            ScreenShareStatus::Live | ScreenShareStatus::Starting
        ) {
            self.stop();
        } else {
            self.start(on_frame);
        }
    }

    /// Returns the current screen sharing status.
    pub(crate) fn status(&self) -> ScreenShareStatus {
        (self.status)()
    }
}

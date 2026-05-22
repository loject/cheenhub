//! Screen sharing context handle.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;

use super::backend::{
    ScreenShareBackend, ScreenShareCallbacks, ScreenShareConfig, ScreenShareFrameCallback,
    ScreenShareSession, ScreenShareStatus,
};
use super::provider_runtime::{next_generation, status_from_error};

/// Context handle used by features that need screen capture.
#[derive(Clone)]
pub(crate) struct ScreenShareHandle {
    pub(super) status: Signal<ScreenShareStatus>,
    pub(super) session: Signal<Option<Rc<dyn ScreenShareSession>>>,
    pub(super) generation: Signal<u64>,
    pub(super) backend: Rc<dyn ScreenShareBackend>,
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
        let start_generation = next_generation(&mut generation);
        status.set(ScreenShareStatus::Starting);

        info!("starting screen sharing capture");
        spawn(async move {
            let ended_session = Rc::new(RefCell::new(session));
            let ended_status = Rc::new(RefCell::new(status));
            let ended_generation = Rc::new(RefCell::new(generation));
            let callbacks = ScreenShareCallbacks {
                on_frame: on_frame.clone(),
                on_ended: Rc::new(move || {
                    let ended_generation_value =
                        next_generation(&mut ended_generation.borrow_mut());
                    ended_session.borrow_mut().set(None);
                    ended_status.borrow_mut().set(ScreenShareStatus::Idle);
                    info!(
                        generation = ended_generation_value,
                        "screen sharing capture ended by browser"
                    );
                }),
            };
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

//! Microphone context provider.

use std::rc::Rc;

use dioxus::prelude::*;

use super::backend::{
    MicrophoneBackend, MicrophoneConfig, MicrophoneFrameCallback, MicrophoneSession,
    MicrophoneStatus,
};
use super::browser::BrowserMicrophoneBackend;

/// Context handle used by features that need microphone input.
#[derive(Clone)]
pub(crate) struct MicrophoneHandle {
    status: Signal<MicrophoneStatus>,
    session: Signal<Option<Rc<dyn MicrophoneSession>>>,
    backend: Rc<dyn MicrophoneBackend>,
}

impl MicrophoneHandle {
    /// Starts microphone capture with the default configuration.
    pub(crate) fn start(&self, on_frame: MicrophoneFrameCallback) {
        if matches!(
            self.status(),
            MicrophoneStatus::Starting | MicrophoneStatus::Live
        ) {
            return;
        }

        let backend = self.backend.clone();
        let mut session = self.session;
        let mut status = self.status;
        status.set(MicrophoneStatus::Starting);

        spawn(async move {
            match backend.start(MicrophoneConfig::default(), on_frame).await {
                Ok(next_session) => {
                    session.set(Some(next_session));
                    status.set(MicrophoneStatus::Live);
                }
                Err(error) => {
                    let next_status = status_from_error(error.clone());
                    warn!(%error, status = ?next_status, "failed to start microphone capture");
                    session.set(None);
                    status.set(next_status);
                }
            }
        });
    }

    /// Restarts microphone capture with a fresh frame callback.
    pub(crate) fn restart(&self, on_frame: MicrophoneFrameCallback) {
        let previous_session = (self.session)();
        let backend = self.backend.clone();
        let mut session = self.session;
        let mut status = self.status;
        status.set(MicrophoneStatus::Starting);

        spawn(async move {
            if let Some(previous_session) = previous_session
                && let Err(error) = previous_session.stop().await
            {
                warn!(%error, "failed to stop previous microphone capture before restart");
            }

            match backend.start(MicrophoneConfig::default(), on_frame).await {
                Ok(next_session) => {
                    session.set(Some(next_session));
                    status.set(MicrophoneStatus::Live);
                }
                Err(error) => {
                    let next_status = status_from_error(error.clone());
                    warn!(%error, status = ?next_status, "failed to restart microphone capture");
                    session.set(None);
                    status.set(next_status);
                }
            }
        });
    }

    /// Stops the active microphone session.
    pub(crate) fn stop(&self) {
        let Some(active_session) = (self.session)() else {
            let mut status = self.status;
            status.set(MicrophoneStatus::Idle);
            return;
        };

        let mut session = self.session;
        let mut status = self.status;
        spawn(async move {
            if let Err(error) = active_session.stop().await {
                warn!(%error, "failed to stop microphone capture cleanly");
            }
            session.set(None);
            status.set(MicrophoneStatus::Idle);
        });
    }

    /// Toggles microphone capture.
    pub(crate) fn toggle(&self, on_frame: MicrophoneFrameCallback) {
        if matches!(
            self.status(),
            MicrophoneStatus::Live | MicrophoneStatus::Starting
        ) {
            self.stop();
        } else {
            self.start(on_frame);
        }
    }

    /// Returns the current microphone status.
    pub(crate) fn status(&self) -> MicrophoneStatus {
        (self.status)()
    }

    /// Updates the active encoder bitrate.
    #[allow(dead_code)]
    pub(crate) fn set_bitrate_bps(&self, bitrate_bps: u32) {
        let Some(active_session) = (self.session)() else {
            return;
        };

        spawn(async move {
            if let Err(error) = active_session.set_bitrate_bps(bitrate_bps).await {
                warn!(%error, bitrate_bps, "failed to update microphone bitrate");
            }
        });
    }
}

fn status_from_error(error: super::backend::MicrophoneError) -> MicrophoneStatus {
    if error.is_permission_denied() {
        MicrophoneStatus::PermissionDenied
    } else {
        MicrophoneStatus::Error(error.to_string())
    }
}

/// Provides microphone capture state to authenticated app components.
#[component]
pub(crate) fn MicrophoneProvider(children: Element) -> Element {
    let status = use_signal(|| MicrophoneStatus::Idle);
    let session = use_signal(|| None::<Rc<dyn MicrophoneSession>>);
    let backend: Rc<dyn MicrophoneBackend> = Rc::new(BrowserMicrophoneBackend);
    let handle = MicrophoneHandle {
        status,
        session,
        backend,
    };
    use_context_provider(move || handle.clone());

    rsx! {
        {children}
    }
}

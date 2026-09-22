//! Контекстный хэндл демонстрации экрана.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::core::{Runtime, ScopeId};
use dioxus::prelude::*;

use crate::features::toast::ToastHandle;

use super::backend::{
    ScreenShareBackend, ScreenShareCaptureSource, ScreenShareConfig, ScreenShareFrameCallback,
    ScreenShareSession, ScreenShareStartRequest, ScreenShareStatus,
};
use super::picker_flow::{
    ScreenShareControlIntent, capture_source_from_selection, screen_share_control_intent,
};
use super::provider_runtime::{next_generation, screen_share_callbacks, status_from_error};
use super::source_picker::ScreenShareSelection;
use super::source_preview;

/// Контекстный хэндл, используемый функциями, которым нужен захват экрана.
#[derive(Clone)]
pub(crate) struct ScreenShareHandle {
    pub(super) provider_scope: ScopeId,
    pub(super) status: Signal<ScreenShareStatus>,
    pub(super) session: Signal<Option<Rc<dyn ScreenShareSession>>>,
    pub(super) generation: Signal<u64>,
    pub(super) backend: Rc<dyn ScreenShareBackend>,
    pub(super) toast: ToastHandle,
    pub(super) picker_open: Signal<bool>,
    pub(super) pending_on_frame: Rc<RefCell<Option<ScreenShareFrameCallback>>>,
}

impl ScreenShareHandle {
    /// Запускает демонстрацию экрана с конфигурацией по умолчанию.
    pub(crate) fn start(&self, on_frame: ScreenShareFrameCallback) {
        self.start_request(
            ScreenShareStartRequest {
                config: ScreenShareConfig::default(),
                source: ScreenShareCaptureSource::PlatformDefault,
            },
            on_frame,
        );
    }

    /// Запускает демонстрацию экрана после подтверждённого выбора источника.
    pub(super) fn start_selected(&self, selection: &ScreenShareSelection) {
        let Some(on_frame) = self.pending_on_frame.borrow_mut().take() else {
            warn!("screen share picker confirmed without a pending frame callback");
            let mut picker_open = self.picker_open;
            picker_open.set(false);
            return;
        };

        self.start_request(
            ScreenShareStartRequest {
                config: ScreenShareConfig::default(),
                source: capture_source_from_selection(selection),
            },
            on_frame,
        );
    }

    /// Очищает callback, ожидающий подтверждения выбора источника.
    pub(super) fn clear_pending_on_frame(&self) {
        self.pending_on_frame.borrow_mut().take();
    }

    fn start_request(&self, request: ScreenShareStartRequest, on_frame: ScreenShareFrameCallback) {
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
        let provider_scope = self.provider_scope;
        let start_generation = next_generation(&mut generation);
        status.set(ScreenShareStatus::Starting);

        info!(source = ?request.source, "starting screen sharing capture");
        debug!("scheduling screen sharing lifecycle task on provider scope");
        Runtime::current().in_scope(provider_scope, || {
            spawn(async move {
            debug!("screen sharing lifecycle task started");
            let callbacks = screen_share_callbacks(
                on_frame.clone(),
                session,
                status,
                generation,
                start_generation,
                provider_scope,
            );
            debug!("calling screen sharing backend start");
            match backend.start(request, callbacks).await {
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
        let provider_scope = self.provider_scope;
        info!("stopping screen sharing capture");
        debug!("scheduling screen sharing stop task on provider scope");
        Runtime::current().in_scope(provider_scope, || {
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
        });
    }

    /// Toggles screen sharing capture.
    pub(crate) fn toggle(&self, on_frame: ScreenShareFrameCallback) {
        match screen_share_control_intent(source_preview::selection_available(), &self.status()) {
            ScreenShareControlIntent::OpenPicker => {
                *self.pending_on_frame.borrow_mut() = Some(on_frame);
                let mut picker_open = self.picker_open;
                picker_open.set(true);
                info!("opened screen share source picker");
            }
            ScreenShareControlIntent::Start => self.start(on_frame),
            ScreenShareControlIntent::Stop => self.stop(),
        }
    }

    /// Returns the current screen sharing status.
    pub(crate) fn status(&self) -> ScreenShareStatus {
        (self.status)()
    }
}

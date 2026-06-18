//! Контекстный хэндл камеры.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;

use crate::features::toast::ToastHandle;

use super::backend::{
    CameraBackend, CameraCallbacks, CameraConfig, CameraFrameCallback, CameraSession, CameraStatus,
};
use super::provider_runtime::{next_generation, status_from_error};

/// Контекстный хэндл, используемый функциями, которым нужен захват камеры.
#[derive(Clone)]
pub(crate) struct CameraHandle {
    pub(super) status: Signal<CameraStatus>,
    pub(super) session: Signal<Option<Rc<dyn CameraSession>>>,
    pub(super) generation: Signal<u64>,
    pub(super) backend: Rc<dyn CameraBackend>,
    pub(super) toast: ToastHandle,
}

impl CameraHandle {
    /// Запускает захват камеры с конфигурацией по умолчанию.
    pub(crate) fn start(&self, on_frame: CameraFrameCallback) {
        if matches!(self.status(), CameraStatus::Starting | CameraStatus::Live) {
            return;
        }

        let backend = self.backend.clone();
        let mut session = self.session;
        let mut status = self.status;
        let mut generation = self.generation;
        let toast = self.toast;
        let start_generation = next_generation(&mut generation);
        status.set(CameraStatus::Starting);

        info!("starting camera capture");
        spawn(async move {
            let ended_session = Rc::new(RefCell::new(session));
            let ended_status = Rc::new(RefCell::new(status));
            let ended_generation = Rc::new(RefCell::new(generation));
            let callbacks = CameraCallbacks {
                on_frame: on_frame.clone(),
                on_ended: Rc::new(move || {
                    let ended_generation_value =
                        next_generation(&mut ended_generation.borrow_mut());
                    ended_session.borrow_mut().set(None);
                    ended_status.borrow_mut().set(CameraStatus::Idle);
                    info!(
                        generation = ended_generation_value,
                        "camera capture ended by browser"
                    );
                }),
            };
            match backend.start(CameraConfig::default(), callbacks).await {
                Ok(next_session) => {
                    if generation() != start_generation {
                        if let Err(error) = next_session.stop().await {
                            warn!(%error, "failed to stop stale camera capture after start");
                        }
                        return;
                    }
                    session.set(Some(next_session));
                    status.set(CameraStatus::Live);
                    info!("camera capture started");
                }
                Err(error) => {
                    if generation() != start_generation {
                        return;
                    }
                    let next_status = status_from_error(error.clone());
                    warn!(%error, status = ?next_status, "failed to start camera capture");
                    if error.is_unsupported_browser() {
                        toast.warning(error.to_string());
                    }
                    session.set(None);
                    status.set(next_status);
                }
            }
        });
    }

    /// Останавливает активную сессию камеры.
    pub(crate) fn stop(&self) {
        let mut generation = self.generation;
        let stop_generation = next_generation(&mut generation);
        let Some(active_session) = self.session.peek().clone() else {
            let mut status = self.status;
            status.set(CameraStatus::Idle);
            return;
        };

        let mut session = self.session;
        let mut status = self.status;
        info!("stopping camera capture");
        spawn(async move {
            if let Err(error) = active_session.stop().await {
                warn!(%error, "failed to stop camera capture cleanly");
            }
            if generation() != stop_generation {
                return;
            }
            session.set(None);
            status.set(CameraStatus::Idle);
            info!("camera capture stopped");
        });
    }

    /// Переключает захват камеры.
    pub(crate) fn toggle(&self, on_frame: CameraFrameCallback) {
        if matches!(self.status(), CameraStatus::Live | CameraStatus::Starting) {
            self.stop();
        } else {
            self.start(on_frame);
        }
    }

    /// Возвращает текущее состояние камеры.
    pub(crate) fn status(&self) -> CameraStatus {
        (self.status)()
    }
}

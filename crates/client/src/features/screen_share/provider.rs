//! Контекстный хэндл демонстрации экрана.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::core::{Runtime, ScopeId};
use dioxus::prelude::*;

use crate::features::toast::ToastHandle;

use super::backend::{
    ScreenShareBackend, ScreenShareCaptureSource, ScreenShareConfig, ScreenShareFrameCallback,
    ScreenShareSession, ScreenShareStartControl, ScreenShareStartRequest, ScreenShareStatus,
};
use super::picker_flow::{
    ScreenShareControlIntent, capture_source_from_selection, screen_share_control_intent,
};
use super::provider_runtime::{next_generation, screen_share_callbacks, status_from_error};
use super::source_picker::ScreenShareSelection;
use super::source_preview;

#[cfg(test)]
mod tests;

/// Контекстный хэндл, используемый функциями, которым нужен захват экрана.
#[derive(Clone)]
pub(crate) struct ScreenShareHandle {
    /// Состояние реально захваченного аудио текущей демонстрации.
    pub(super) audio_state: Signal<super::backend::ScreenShareAudioState>,
    pub(super) provider_scope: ScopeId,
    pub(super) status: Signal<ScreenShareStatus>,
    pub(super) session: Signal<Option<Rc<dyn ScreenShareSession>>>,
    pub(super) generation: Signal<u64>,
    pub(super) start_control: Signal<Option<ScreenShareStartControl>>,
    pub(super) backend: Rc<dyn ScreenShareBackend>,
    pub(super) toast: ToastHandle,
    pub(super) picker_open: Signal<bool>,
    pub(super) last_selection: Signal<Option<ScreenShareSelection>>,
    pub(super) pending_on_frame: Rc<RefCell<Option<ScreenShareFrameCallback>>>,
}

impl ScreenShareHandle {
    /// Возвращает UUID реально активного системного звука демонстрации.
    pub(crate) fn audio_track_id(&self) -> Option<uuid::Uuid> {
        match (self.audio_state)() {
            super::backend::ScreenShareAudioState::Live { track_id, .. } => Some(track_id),
            _ => None,
        }
    }

    /// Проверяет активность UUID без подписки фонового отправителя на Dioxus signal.
    pub(crate) fn audio_track_is_active(&self, track_id: uuid::Uuid) -> bool {
        matches!(&*self.audio_state.peek(), super::backend::ScreenShareAudioState::Live { track_id: active, .. } if *active == track_id)
    }

    /// Забирает очередь аудио текущей сессии; чтение подписывает resource на смену сессии.
    pub(crate) fn take_audio_frames(
        &self,
    ) -> Option<futures_channel::mpsc::Receiver<super::backend::EncodedScreenAudioFrame>> {
        self.session.read().as_ref()?.take_audio_frames()
    }
    /// Запускает демонстрацию экрана с конфигурацией по умолчанию.
    pub(crate) fn start(&self, on_frame: ScreenShareFrameCallback) {
        self.start_request(
            ScreenShareStartRequest {
                config: ScreenShareConfig::default(),
                source: ScreenShareCaptureSource::PlatformDefault { target: None },
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
                config: ScreenShareConfig {
                    automatic_bitrate_bps: selection.bitrate_bps,
                    ..ScreenShareConfig::default()
                },
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
        let mut audio_state = self.audio_state;
        audio_state.set(super::backend::ScreenShareAudioState::Off);
        let start_control = ScreenShareStartControl::default();
        let mut pending_start = self.start_control;
        pending_start.set(Some(start_control.clone()));
        let start_generation = next_generation(&mut generation);
        status.set(ScreenShareStatus::Starting);

        info!(source = ?request.source, "starting screen sharing capture");
        let callbacks = screen_share_callbacks(
            on_frame,
            super::provider_runtime::CaptureRuntime {
                session,
                status,
                generation,
                start_generation,
                provider_scope,
                audio_state,
                start_control: start_control.clone(),
                pending_start,
                on_warning: Rc::new(move |message| toast.warning(message.to_owned())),
            },
        );
        let (result_sender, result_receiver) = futures_channel::oneshot::channel();
        let result_control = start_control.clone();

        debug!("scheduling screen sharing result handler on provider scope");
        Runtime::current().in_scope(provider_scope, || {
            spawn(async move {
                let Ok(result) = result_receiver.await else {
                    return;
                };
                if result_control.is_cancelled() {
                    stop_unobserved_session(result).await;
                    return;
                }

                match result {
                    Ok(next_session) => {
                        if generation() != start_generation {
                            if let Err(error) = next_session.stop().await {
                                warn!(%error, "failed to stop stale screen sharing capture after start");
                            }
                            return;
                        }
                        pending_start.set(None);
                        session.set(Some(next_session));
                        status.set(ScreenShareStatus::Live);
                        info!("screen sharing capture started");
                    }
                    Err(error) => {
                        if generation() != start_generation {
                            return;
                        }
                        pending_start.set(None);
                        let next_status = status_from_error(error.clone());
                        warn!(%error, status = ?next_status, "failed to start screen sharing capture");
                        if error.is_unsupported_browser() {
                            toast.warning(error.to_string());
                        } else if !error.is_permission_denied() {
                            toast.warning("Не удалось запустить демонстрацию с выбранными настройками.".to_owned());
                        }
                        session.set(None);
                        status.set(next_status);
                    }
                }
            });
        });

        debug!("scheduling backend capture task on application root scope");
        // getDisplayMedia нельзя отменить. Задача в корневом scope должна дождаться позднего ответа
        // и освободить поток, даже если provider scope уже уничтожен.
        Runtime::current().in_scope(ScopeId::ROOT, || {
            spawn(async move {
                debug!("screen sharing backend task started");
                if start_control.is_cancelled() {
                    debug!("discarded screen sharing task after provider scope was dropped");
                    return;
                }
                debug!("calling screen sharing backend start");
                let result = backend.start(request, callbacks).await;
                deliver_start_result(result, start_control, result_sender).await;
            });
        });
    }

    /// Stops the active screen sharing session.
    pub(crate) fn stop(&self) {
        if self.status() == ScreenShareStatus::Starting
            && let Some(control) = self.start_control.peek().clone()
        {
            info!(
                reason = "manual-start-cancel",
                "cancelling screen sharing startup"
            );
            control.cancel();
        }
        let mut audio_state = self.audio_state;
        audio_state.set(super::backend::ScreenShareAudioState::Off);
        let mut generation = self.generation;
        let stop_generation = next_generation(&mut generation);
        let Some(active_session) = self.session.peek().clone() else {
            let mut pending_start = self.start_control;
            pending_start.set(None);
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

    /// Сообщает UI, что качество выбирается перед системным окном источника.
    pub(super) fn supports_platform_quality_picker(&self) -> bool {
        self.backend.supports_platform_quality_picker()
    }

    /// Toggles screen sharing capture.
    pub(crate) fn toggle(&self, on_frame: ScreenShareFrameCallback) {
        let picker_available =
            source_preview::selection_available() || self.supports_platform_quality_picker();
        match screen_share_control_intent(picker_available, &self.status()) {
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

async fn deliver_start_result(
    result: Result<Rc<dyn ScreenShareSession>, super::backend::ScreenShareError>,
    start_control: ScreenShareStartControl,
    result_sender: futures_channel::oneshot::Sender<
        Result<Rc<dyn ScreenShareSession>, super::backend::ScreenShareError>,
    >,
) {
    if start_control.is_cancelled() {
        stop_unobserved_session(result).await;
        return;
    }
    if let Err(result) = result_sender.send(result) {
        stop_unobserved_session(result).await;
    }
}

async fn stop_unobserved_session(
    result: Result<Rc<dyn ScreenShareSession>, super::backend::ScreenShareError>,
) {
    if let Ok(session) = result
        && let Err(error) = session.stop().await
    {
        warn!(%error, "failed to stop screen sharing capture after its provider disappeared");
    }
}

//! Применение денойзера вне очереди команд с отменой и ограниченным ожиданием.

use std::future::Future;
use std::sync::atomic::Ordering;

use dioxus::prelude::*;
use futures_channel::oneshot;
use futures_util::future::{Either, select};

use super::{MicrophoneRuntime, clear_failed_capture, operation_is_current};
use crate::features::microphone::{
    MicrophoneError, MicrophoneStatus, denoise::DenoiseMode, storage,
};

/// Отменяет ожидание текущего применения при Stop или смене capture-сессии.
pub(super) fn cancel(runtime: &MicrophoneRuntime) {
    if let Some(cancel) = runtime.denoise_cancel.borrow_mut().take() {
        let _ = cancel.send(());
    }
    let mut pending = runtime.denoise_pending;
    pending.set(false);
}

/// Запускает применение отдельно от очереди и сохраняет предпочтение только при успехе.
pub(super) fn apply(runtime: MicrophoneRuntime, mode: DenoiseMode) {
    apply_with_persistence(runtime, mode, storage::save_denoise_mode);
}

fn apply_with_persistence(
    mut runtime: MicrophoneRuntime,
    mode: DenoiseMode,
    persist: impl FnOnce(DenoiseMode) + 'static,
) {
    cancel(&runtime);
    runtime.denoise_pending.set(true);
    let generation = *runtime.generation.peek();
    let session = runtime.session.peek().clone();
    if session.is_none() && matches!(*runtime.status.peek(), MicrophoneStatus::Starting) {
        runtime.denoise_pending.set(false);
        runtime
            .toast
            .warning("Дождитесь включения микрофона и попробуйте ещё раз.");
        return;
    }
    let (sender, cancelled) = oneshot::channel();
    *runtime.denoise_cancel.borrow_mut() = Some(sender);
    spawn(async move {
        let update = async {
            if let Some(session) = &session {
                session
                    .set_denoise_mode(mode, runtime.denoise_runtime.clone())
                    .await
            } else {
                runtime
                    .denoise_runtime
                    .store(mode == DenoiseMode::Rnnoise, Ordering::Relaxed);
                Ok(())
            }
        };
        let result = wait(update, cancelled, (runtime.denoise_timeout)()).await;
        if !operation_is_current(&runtime, generation) {
            return;
        }
        match result {
            Outcome::Applied(Ok(())) => {
                runtime.denoise_mode.set(mode);
                runtime.denoise_failed.set(runtime.denoise_runtime.failed());
                persist(mode);
                info!(?mode, generation, "microphone noise suppression committed");
            }
            Outcome::Applied(Err(error)) => {
                if error.is_invalidated() {
                    if let Some(session) = &session {
                        session.stop_immediately();
                        let session = session.clone();
                        spawn(async move {
                            if let Err(error) = session.stop().await {
                                warn!(%error, generation, "microphone invalidated session cleanup failed");
                            }
                        });
                    }
                    let mut generation_signal = runtime.generation;
                    super::next_generation(&mut generation_signal);
                    clear_failed_capture(&runtime, MicrophoneStatus::Error(error.to_string()));
                }
                warn!(%error, ?mode, generation, rolled_back = !error.is_invalidated(), "microphone noise suppression update failed");
                if error.is_invalidated() {
                    runtime.toast.warning("Микрофон остановлен: не удалось восстановить обработку звука. Включите его снова.");
                } else {
                    runtime
                        .toast
                        .warning("Не удалось изменить подавление шума. Сохранён предыдущий режим.");
                }
            }
            Outcome::TimedOut => {
                // applyConstraints нельзя отменить: отзываем track, чтобы поздний результат не изменил действующую сессию.
                if let Some(session) = &session {
                    session.stop_immediately();
                    let session = session.clone();
                    spawn(async move {
                        if let Err(error) = session.stop().await {
                            warn!(%error, generation, "microphone invalidated session cleanup failed");
                        }
                    });
                }
                let mut generation_signal = runtime.generation;
                super::next_generation(&mut generation_signal);
                clear_failed_capture(&runtime, MicrophoneStatus::Error("Изменение подавления шума заняло слишком много времени. Включите микрофон снова.".into()));
                warn!(
                    generation,
                    "microphone denoise update timed out; capture revoked"
                );
                runtime.toast.warning("Микрофон остановлен: изменение подавления шума не завершилось. Включите его снова.");
            }
            Outcome::Cancelled => return,
        }
        runtime.denoise_cancel.borrow_mut().take();
        runtime.denoise_pending.set(false);
    });
}

enum Outcome {
    Applied(Result<(), MicrophoneError>),
    Cancelled,
    TimedOut,
}

async fn wait(
    update: impl Future<Output = Result<(), MicrophoneError>>,
    cancelled: oneshot::Receiver<()>,
    timeout: impl Future<Output = ()>,
) -> Outcome {
    match select(
        Box::pin(update),
        Box::pin(select(cancelled, Box::pin(timeout))),
    )
    .await
    {
        Either::Left((result, _)) => Outcome::Applied(result),
        Either::Right((Either::Left(_), _)) => Outcome::Cancelled,
        Either::Right((Either::Right(_), _)) => Outcome::TimedOut,
    }
}

#[cfg(test)]
mod tests;

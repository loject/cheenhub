//! Живое изменение browser constraints с rollback и ограниченным ожиданием.

use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, atomic::Ordering},
};

use futures_util::{
    FutureExt,
    future::{Either, LocalBoxFuture, select},
};
use web_sys::MediaStreamTrack;

use crate::features::microphone::{
    backend::MicrophoneError,
    browser_worker::BrowserWorkerUplink,
    denoise::{DenoiseMode, RuntimeSwitch},
};

/// Управляет денойзером одного track; новый capture получает отдельный экземпляр.
pub(in crate::features::microphone) struct BrowserDenoiseControl {
    track: MediaStreamTrack,
    uplink: Option<Rc<BrowserWorkerUplink>>,
    closed: Rc<Cell<bool>>,
    mode: Rc<Cell<DenoiseMode>>,
}

impl BrowserDenoiseControl {
    /// Привязывает управление к track и признаку отзыва одной capture-сессии.
    pub(in crate::features::microphone) fn new(
        track: MediaStreamTrack,
        uplink: Option<Rc<BrowserWorkerUplink>>,
        closed: Rc<Cell<bool>>,
        mode: DenoiseMode,
    ) -> Self {
        Self {
            track,
            uplink,
            closed,
            mode: Rc::new(Cell::new(mode)),
        }
    }

    /// Применяет constraints и DSP с rollback; timeout отзывает track вместо позднего применения.
    pub(in crate::features::microphone) fn set(
        &self,
        mode: DenoiseMode,
        runtime: Arc<RuntimeSwitch>,
    ) -> LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        let track = self.track.clone();
        let uplink = self.uplink.clone();
        let closed = self.closed.clone();
        let current = self.mode.clone();
        async move {
            let previous = current.get();
            let operation = async {
                constraints(mode, previous, |mode| {
                    super::super::browser_capture::update_noise_suppression(
                        &track,
                        mode == DenoiseMode::Browser,
                    )
                })
                .await?;
                if closed.get() {
                    return Err(MicrophoneError::invalidated("Микрофон уже остановлен."));
                }
                let snapshot = runtime.snapshot();
                runtime.store(mode == DenoiseMode::Rnnoise, Ordering::Relaxed);
                if let Some(uplink) = &uplink
                    && let Err(error) = uplink
                        .set_denoise_enabled(mode == DenoiseMode::Rnnoise, runtime.snapshot())
                        .await
                {
                    constraints(previous, mode, |mode| {
                        super::super::browser_capture::update_noise_suppression(
                            &track,
                            mode == DenoiseMode::Browser,
                        )
                    })
                    .await
                    .map_err(|_| {
                        MicrophoneError::invalidated("Не удалось восстановить обработку микрофона.")
                    })?;
                    runtime.restore(snapshot);
                    uplink
                        .set_denoise_enabled(runtime.load(Ordering::Relaxed), runtime.snapshot())
                        .await
                        .map_err(|_| {
                            MicrophoneError::invalidated(
                                "Не удалось восстановить обработку микрофона.",
                            )
                        })?;
                    return Err(error);
                }
                if closed.get() {
                    return Err(MicrophoneError::invalidated("Микрофон уже остановлен."));
                }
                current.set(mode);
                Ok(())
            };
            match select(
                Box::pin(operation),
                Box::pin(crate::features::runtime::sleep_ms(3_000)),
            )
            .await
            {
                Either::Left((result, _)) => {
                    if result.as_ref().is_err_and(MicrophoneError::is_invalidated) {
                        track.stop();
                        if let Some(uplink) = &uplink {
                            uplink.stop();
                        }
                    }
                    result
                }
                Either::Right(_) => {
                    track.stop();
                    if let Some(uplink) = &uplink {
                        uplink.stop();
                    }
                    Err(MicrophoneError::invalidated(
                        "Изменение подавления шума не завершилось вовремя.",
                    ))
                }
            }
        }
        .boxed_local()
    }
}

async fn constraints<F, Fut>(
    next: DenoiseMode,
    previous: DenoiseMode,
    mut apply: F,
) -> Result<(), MicrophoneError>
where
    F: FnMut(DenoiseMode) -> Fut,
    Fut: std::future::Future<Output = Result<(), MicrophoneError>>,
{
    if (next == DenoiseMode::Browser) == (previous == DenoiseMode::Browser) {
        return Ok(());
    }
    if let Err(error) = apply(next).await {
        apply(previous).await.map_err(|_| {
            MicrophoneError::invalidated("Не удалось восстановить обработку микрофона.")
        })?;
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests;

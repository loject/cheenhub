//! Linux-захват микрофона через PulseAudio API, включая совместимый сервер PipeWire.

mod worker;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use dioxus::prelude::{debug, info, warn};
use futures_channel::oneshot;
use futures_util::FutureExt;

use super::super::backend::{
    MicrophoneBackend, MicrophoneCallbacks, MicrophoneCodec, MicrophoneConfig, MicrophoneError,
    MicrophoneSession,
};
use super::encoding::{
    NativeMicrophoneEvent, frame_samples, spawn_encoder_worker, spawn_event_relay,
};

/// Захватывает Linux-микрофон через звуковой сервер пользователя.
#[derive(Debug, Default)]
pub(super) struct PulseMicrophoneBackend;

struct Cancellation(Arc<AtomicBool>);

impl Drop for Cancellation {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

struct PulseMicrophoneSession {
    cancellation: Cancellation,
    bitrate_bps: Arc<AtomicU32>,
    input_gain_bits: Arc<AtomicU32>,
    finished: RefCell<Option<oneshot::Receiver<()>>>,
}

impl MicrophoneSession for PulseMicrophoneSession {
    fn stop_immediately(&self) {
        self.cancellation.0.store(true, Ordering::Relaxed);
    }

    fn stop(&self) -> futures_util::future::LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        self.stop_immediately();
        let finished = self.finished.borrow_mut().take();
        async move {
            if let Some(finished) = finished {
                let _ = finished.await;
            }
            info!("linux microphone capture stopped");
            Ok(())
        }
        .boxed_local()
    }

    fn set_bitrate_bps(
        &self,
        bitrate_bps: u32,
    ) -> futures_util::future::LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        self.bitrate_bps.store(bitrate_bps, Ordering::Relaxed);
        debug!(bitrate_bps, "linux microphone bitrate update queued");
        async { Ok(()) }.boxed_local()
    }

    fn set_input_gain(&self, input_gain: f32) -> Result<(), MicrophoneError> {
        self.input_gain_bits
            .store(input_gain.clamp(0.0, 2.0).to_bits(), Ordering::Relaxed);
        debug!(input_gain, "linux microphone input gain updated");
        Ok(())
    }
}

impl MicrophoneBackend for PulseMicrophoneBackend {
    fn start(
        &self,
        config: MicrophoneConfig,
        callbacks: MicrophoneCallbacks,
    ) -> futures_util::future::LocalBoxFuture<
        'static,
        Result<Rc<dyn MicrophoneSession>, MicrophoneError>,
    > {
        async move {
            if config.codec != MicrophoneCodec::Opus
                || config.channels != 1
                || config.sample_rate_hz != 48_000
            {
                return Err(MicrophoneError::new(
                    "Микрофон поддерживает Opus, mono, 48000 Гц.",
                ));
            }
            let closed = Arc::new(AtomicBool::new(false));
            // Отмена future запуска освобождает worker даже до получения сессии.
            let cancellation = Cancellation(closed.clone());
            let bitrate_bps = Arc::new(AtomicU32::new(config.bitrate_bps));
            let input_gain_bits = Arc::new(AtomicU32::new(config.input_gain.to_bits()));
            let (pcm_sender, pcm_receiver) = std::sync::mpsc::sync_channel(48);
            let (event_sender, event_receiver) = futures_channel::mpsc::unbounded();
            let (started_sender, started_receiver) = oneshot::channel();
            let (finished_sender, finished_receiver) = oneshot::channel();
            let source = config.device_id.clone().filter(|id| !id.is_empty());
            let worker_closed = closed.clone();
            let worker_events = event_sender.clone();
            std::thread::Builder::new()
                .name("cheenhub-pulse-capture".to_owned())
                .spawn(move || {
                    let result = worker::run(
                        source.as_deref(),
                        &worker_closed,
                        pcm_sender,
                        started_sender,
                    );
                    if let Err(error) = result {
                        warn!(%error, "linux microphone capture failed");
                        let _ = worker_events.unbounded_send(NativeMicrophoneEvent::Error(error));
                    }
                    worker_closed.store(true, Ordering::Relaxed);
                    let _ = finished_sender.send(());
                })
                .map_err(|error| {
                    warn!(%error, "linux microphone worker spawn failed");
                    MicrophoneError::new("Не удалось запустить микрофон. Попробуйте ещё раз.")
                })?;
            started_receiver
                .await
                .map_err(|_| MicrophoneError::new("Запуск микрофона прерван."))??;
            spawn_event_relay(event_receiver, callbacks);
            let samples = frame_samples(config.sample_rate_hz);
            spawn_encoder_worker(
                config,
                pcm_receiver,
                event_sender,
                closed,
                bitrate_bps.clone(),
                input_gain_bits.clone(),
                samples,
            );
            info!("linux microphone PulseAudio capture started");
            Ok(Rc::new(PulseMicrophoneSession {
                cancellation,
                bitrate_bps,
                input_gain_bits,
                finished: RefCell::new(Some(finished_receiver)),
            }) as Rc<dyn MicrophoneSession>)
        }
        .boxed_local()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropping_startup_guard_requests_worker_shutdown() {
        let closed = Arc::new(AtomicBool::new(false));
        drop(Cancellation(closed.clone()));
        assert!(closed.load(Ordering::Relaxed));
    }
}

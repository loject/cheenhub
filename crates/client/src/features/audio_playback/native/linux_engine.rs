//! Вывод Linux через PulseAudio API, совместимый с PipeWire.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use dioxus::prelude::{debug, info, warn};
use libpulse_binding as pulse;

use super::cpal_playback::mixer::{MixerHandle, NativeOutputMixer, new_mixer};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);
const RETRY_DELAY: Duration = Duration::from_secs(5);

/// Поток звукового сервера и разделяемый микшер.
pub(super) struct NativePlaybackEngine {
    /// Очереди декодированного звука и настройки громкости.
    pub(super) mixer: MixerHandle,
    device_id: Option<String>,
    stop: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    created_at: Instant,
}

impl NativePlaybackEngine {
    /// Возвращает сохранённое назначение потока.
    pub(super) fn device_id(&self) -> Option<String> {
        self.device_id.clone()
    }

    /// Разрешает повторный запуск завершённого worker после защитного интервала.
    pub(super) fn is_running(&self) -> bool {
        self.created_at.elapsed() < RETRY_DELAY
            || self
                .worker
                .as_ref()
                .is_some_and(|worker| !worker.is_finished())
    }
}

impl Drop for NativePlaybackEngine {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            warn!("PulseAudio playback worker panicked");
        }
        debug!(
            was_ready = self.ready.load(Ordering::Acquire),
            "PulseAudio playback worker stopped"
        );
    }
}

/// Запускает подключение в отдельном потоке без ожидания звукового сервера в UI.
pub(super) fn create_engine(
    device_id: Option<String>,
    output_gain: f32,
    sample_rate_hz: u32,
) -> Result<NativePlaybackEngine, String> {
    if device_id.as_deref().is_some_and(|id| id.contains('\0')) {
        return Err("Некорректный идентификатор устройства вывода.".into());
    }
    let mixer = new_mixer(output_gain);
    let stop = Arc::new(AtomicBool::new(false));
    let ready = Arc::new(AtomicBool::new(false));
    let worker_ready = ready.clone();
    let worker_stop = stop.clone();
    let worker_mixer = mixer.clone();
    let selected_device = device_id.clone();
    let worker = thread::Builder::new().name("pulse-playback".into()).spawn(move || {
        if let Err(error) = run(selected_device.as_deref(), sample_rate_hz, worker_mixer, &worker_stop, &worker_ready) {
            warn!(%error, selected_device = selected_device.as_deref().unwrap_or(""), "PulseAudio playback failed");
        }
    }).map_err(|error| format!("Не удалось запустить воспроизведение: {error}"))?;
    Ok(NativePlaybackEngine {
        mixer,
        device_id,
        stop,
        ready,
        worker: Some(worker),
        created_at: Instant::now(),
    })
}

fn run(
    device: Option<&str>,
    rate: u32,
    mixer: MixerHandle,
    stop: &AtomicBool,
    ready_signal: &AtomicBool,
) -> Result<(), String> {
    let mut mainloop = pulse::mainloop::standard::Mainloop::new()
        .ok_or("Не удалось создать PulseAudio mainloop.")?;
    let mut context = pulse::context::Context::new(&mainloop, "CheenHub")
        .ok_or("Не удалось создать PulseAudio context.")?;
    let result = (|| {
        context
            .connect(None, pulse::context::FlagSet::NOAUTOSPAWN, None)
            .map_err(pulse_error)?;
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        loop {
            tick(&mut mainloop)?;
            if stop.load(Ordering::Acquire) {
                return Ok(());
            }
            match context.get_state() {
                pulse::context::State::Ready => break,
                pulse::context::State::Failed | pulse::context::State::Terminated => {
                    return Err(pulse_error(context.errno()));
                }
                _ if Instant::now() >= deadline => {
                    return Err("Звуковой сервер не ответил вовремя.".into());
                }
                _ => thread::sleep(Duration::from_millis(3)),
            }
        }
        let spec = pulse::sample::Spec {
            format: pulse::sample::Format::F32le,
            channels: 2,
            rate,
        };
        let mut stream = pulse::stream::Stream::new(&mut context, "CheenHub playback", &spec, None)
            .ok_or("Не удалось создать поток воспроизведения.")?;
        let attr = pulse::def::BufferAttr {
            maxlength: u32::MAX,
            tlength: rate / 50 * 8,
            prebuf: u32::MAX,
            minreq: rate / 200 * 8,
            fragsize: u32::MAX,
        };
        stream
            .connect_playback(
                device,
                Some(&attr),
                pulse::stream::FlagSet::ADJUST_LATENCY
                    | if device.is_some() {
                        pulse::stream::FlagSet::DONT_MOVE
                    } else {
                        pulse::stream::FlagSet::empty()
                    },
                None,
                None,
            )
            .map_err(pulse_error)?;
        let mut output = NativeOutputMixer::new(rate, rate, mixer);
        let mut ready = false;
        let mut bytes = Vec::with_capacity(8192);
        while !stop.load(Ordering::Acquire) {
            tick(&mut mainloop)?;
            if matches!(
                context.get_state(),
                pulse::context::State::Failed | pulse::context::State::Terminated
            ) {
                return Err(pulse_error(context.errno()));
            }
            match stream.get_state() {
                pulse::stream::State::Ready => {
                    if let Some(selected) = device
                        && stream.get_device_name().as_deref() != Some(selected)
                    {
                        return Err("Выбранное устройство вывода недоступно.".into());
                    }
                    if !ready {
                        info!(
                            sample_rate_hz = rate,
                            selected_device = device.unwrap_or(""),
                            "PulseAudio playback started"
                        );
                        ready = true;
                        ready_signal.store(true, Ordering::Release);
                    }
                    let frames = stream.writable_size().unwrap_or(0).min(8192) / 8;
                    if frames != 0 {
                        bytes.clear();
                        output.render_frames(frames, |_, sample| {
                            let sample = sample.clamp(-1.0, 1.0).to_le_bytes();
                            bytes.extend_from_slice(&sample);
                            bytes.extend_from_slice(&sample);
                        });
                        stream
                            .write_copy(&bytes, 0, pulse::stream::SeekMode::Relative)
                            .map_err(pulse_error)?;
                    }
                }
                pulse::stream::State::Failed | pulse::stream::State::Terminated => {
                    return Err(pulse_error(context.errno()));
                }
                _ if Instant::now() >= deadline => {
                    return Err("Устройство вывода не ответило вовремя.".into());
                }
                _ => {}
            }
            thread::sleep(Duration::from_millis(3));
        }
        let _ = stream.disconnect();
        Ok(())
    })();
    context.disconnect();
    result
}

fn tick(mainloop: &mut pulse::mainloop::standard::Mainloop) -> Result<(), String> {
    match mainloop.iterate(false) {
        pulse::mainloop::standard::IterateResult::Success(_) => Ok(()),
        _ => Err("Остановлен цикл событий звукового сервера.".into()),
    }
}

fn pulse_error(error: impl std::fmt::Display) -> String {
    format!("Ошибка звукового сервера PulseAudio: {error}")
}

#[cfg(test)]
#[path = "linux_engine_tests.rs"]
mod tests;

//! Вывод Linux через PulseAudio API, совместимый с PipeWire.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use dioxus::prelude::{debug, info, warn};
use libpulse_binding as pulse;

use super::cpal_playback::mixer::{MixerHandle, NativeOutputMixer, new_mixer};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);
const RETRY_DELAY: Duration = Duration::from_secs(5);
const UNDERFLOW_WARNING_INTERVAL: Duration = Duration::from_secs(5);

/// Поток звукового сервера и разделяемый микшер.
pub(super) struct NativePlaybackEngine {
    /// Очереди декодированного звука и настройки громкости.
    pub(super) mixer: MixerHandle,
    device_id: Option<String>,
    stop: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    underflows: Arc<AtomicU64>,
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
            underflows = self.underflows.load(Ordering::Relaxed),
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
    let underflows = Arc::new(AtomicU64::new(0));
    let worker_underflows = underflows.clone();
    let worker_ready = ready.clone();
    let worker_stop = stop.clone();
    let worker_mixer = mixer.clone();
    let selected_device = device_id.clone();
    let worker = thread::Builder::new().name("pulse-playback".into()).spawn(move || {
        if let Err(error) = run(selected_device.as_deref(), sample_rate_hz, worker_mixer, || worker_stop.load(Ordering::Acquire), &worker_ready, worker_underflows) {
            warn!(%error, selected_device = selected_device.as_deref().unwrap_or(""), "PulseAudio playback failed");
        }
    }).map_err(|error| format!("Не удалось запустить воспроизведение: {error}"))?;
    Ok(NativePlaybackEngine {
        mixer,
        device_id,
        stop,
        ready,
        underflows,
        worker: Some(worker),
        created_at: Instant::now(),
    })
}

fn run(
    device: Option<&str>,
    rate: u32,
    mixer: MixerHandle,
    mut should_stop: impl FnMut() -> bool,
    ready_signal: &AtomicBool,
    underflows: Arc<AtomicU64>,
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
            if should_stop() {
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
        let (attr, flags) = playback_buffer_config(rate, device.is_some());
        let mut last_warning = None::<Instant>;
        let callback_underflows = underflows.clone();
        stream.set_underflow_callback(Some(Box::new(move || {
            let count = callback_underflows.fetch_add(1, Ordering::Relaxed) + 1;
            if last_warning.is_none_or(|last| last.elapsed() >= UNDERFLOW_WARNING_INTERVAL) {
                warn!(underflows = count, "PulseAudio playback buffer underrun");
                last_warning = Some(Instant::now());
            }
        })));
        // Stream владеет callback и освобождает его также при раннем выходе по ошибке.
        stream
            .connect_playback(device, Some(&attr), flags, None, None)
            .map_err(pulse_error)?;
        let mut output = NativeOutputMixer::new(rate, rate, mixer);
        let mut ready = false;
        let mut recovered_underflows = 0;
        let mut bytes = Vec::with_capacity(8192);
        while !should_stop() {
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
                        if let Some(attr) = stream.get_buffer_attr() {
                            info!(
                                sample_rate_hz = rate,
                                tlength_bytes = attr.tlength,
                                minreq_bytes = attr.minreq,
                                prebuf_bytes = attr.prebuf,
                                maxlength_bytes = attr.maxlength,
                                "PulseAudio playback buffer negotiated"
                            );
                        }
                        info!(
                            sample_rate_hz = rate,
                            selected_device = device.unwrap_or(""),
                            "PulseAudio playback started"
                        );
                        ready = true;
                        ready_signal.store(true, Ordering::Release);
                    }
                    let count = underflows.load(Ordering::Relaxed);
                    let recovering = count != recovered_underflows;
                    let (frames, seek) = if recovering {
                        // После underrun writable_size не восстанавливает запас потока.
                        // Заново заполняем буфер от текущей позиции чтения сервера.
                        let negotiated = stream.get_buffer_attr().unwrap_or(&attr);
                        let length = negotiated.tlength.min(negotiated.maxlength) as usize;
                        (length / 8, pulse::stream::SeekMode::RelativeOnRead)
                    } else {
                        (
                            stream.writable_size().unwrap_or(0).min(8192) / 8,
                            pulse::stream::SeekMode::Relative,
                        )
                    };
                    if frames != 0 {
                        bytes.clear();
                        output.render_frames(frames, |_, sample| {
                            let sample = sample.clamp(-1.0, 1.0).to_le_bytes();
                            bytes.extend_from_slice(&sample);
                            bytes.extend_from_slice(&sample);
                        });
                        stream.write_copy(&bytes, 0, seek).map_err(pulse_error)?;
                        if recovering {
                            recovered_underflows = count;
                            debug!(
                                underflows = count,
                                buffer_bytes = bytes.len(),
                                "PulseAudio playback buffer recovered"
                            );
                        }
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
        stream.set_underflow_callback(None);
        let _ = stream.disconnect();
        Ok(())
    })();
    context.disconnect();
    result
}

fn playback_buffer_config(
    rate: u32,
    explicit_device: bool,
) -> (pulse::def::BufferAttr, pulse::stream::FlagSet) {
    // 40 мс относятся к буферу потока, а не к общей задержке устройства.
    // Сервер выбирает minreq под свой граф; слишком малый запрос вызывает underrun.
    let attr = pulse::def::BufferAttr {
        maxlength: u32::MAX,
        tlength: rate / 25 * 8,
        prebuf: u32::MAX,
        minreq: u32::MAX,
        fragsize: u32::MAX,
    };
    let flags = if explicit_device {
        pulse::stream::FlagSet::DONT_MOVE
    } else {
        pulse::stream::FlagSet::empty()
    };
    (attr, flags)
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

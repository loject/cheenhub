//! Отменяемый цикл PulseAudio: все объекты соединения принадлежат одному потоку.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError};
use std::time::{Duration, Instant};

use dioxus::prelude::warn;
use futures_channel::oneshot;
use libpulse_binding::{
    context,
    def::BufferAttr,
    mainloop::standard::{IterateResult, Mainloop},
    sample, stream,
};

use super::MicrophoneError;

// Context::drop выполняет только unref; явный disconnect нужен при любой ошибке.
struct ConnectedContext(context::Context);

impl Drop for ConnectedContext {
    fn drop(&mut self) {
        self.0.disconnect();
    }
}

/// Выполняет захват до отмены; уведомляет вызывающего о результате запуска.
pub(super) fn run(
    source: Option<&str>,
    closed: &AtomicBool,
    pcm: SyncSender<super::super::pcm::Chunk>,
    started: oneshot::Sender<Result<(), MicrophoneError>>,
) -> Result<(), MicrophoneError> {
    let mut started = Some(started);
    let result = capture(source, closed, pcm, &mut started);
    if let Some(started) = started {
        let error = result
            .clone()
            .err()
            .unwrap_or_else(|| MicrophoneError::new("Запуск микрофона отменён."));
        let _ = started.send(Err(error));
    }
    result
}

fn capture(
    source: Option<&str>,
    closed: &AtomicBool,
    pcm: SyncSender<super::super::pcm::Chunk>,
    started: &mut Option<oneshot::Sender<Result<(), MicrophoneError>>>,
) -> Result<(), MicrophoneError> {
    if source.is_some_and(|source| source.contains('\0')) {
        return Err(error("Идентификатор микрофона содержит NUL"));
    }
    let mut mainloop =
        Mainloop::new().ok_or_else(|| error("Не удалось создать цикл аудиосервера"))?;
    let mut connection = ConnectedContext(
        context::Context::new(&mainloop, "CheenHub microphone")
            .ok_or_else(|| error("Не удалось создать подключение к аудиосерверу"))?,
    );
    let context = &mut connection.0;
    context
        .connect(None, context::FlagSet::NOAUTOSPAWN, None)
        .map_err(error)?;
    let deadline = Instant::now() + Duration::from_secs(8);
    while context.get_state() != context::State::Ready {
        if closed.load(Ordering::Relaxed) {
            return Ok(());
        }
        check_context(context)?;
        tick(&mut mainloop, Some(deadline))?;
    }
    let spec = sample::Spec {
        format: sample::Format::FLOAT32NE,
        channels: 1,
        rate: 48_000,
    };
    let mut stream = stream::Stream::new(context, "Microphone", &spec, None)
        .ok_or_else(|| error("Не удалось создать поток микрофона"))?;
    let buffer = BufferAttr {
        maxlength: u32::MAX,
        tlength: u32::MAX,
        prebuf: u32::MAX,
        minreq: u32::MAX,
        fragsize: 480 * 4,
    };
    let mut flags = stream::FlagSet::ADJUST_LATENCY;
    if source.is_some() {
        flags |= stream::FlagSet::DONT_MOVE;
    }
    stream
        .connect_record(source, Some(&buffer), flags)
        .map_err(error)?;
    let mut backlog_warned = false;
    while !closed.load(Ordering::Relaxed) {
        check_context(context)?;
        match stream.get_state() {
            stream::State::Failed | stream::State::Terminated => {
                return Err(error(format!(
                    "Поток выбранного микрофона недоступен: {}",
                    context.errno()
                )));
            }
            stream::State::Ready => {
                if source
                    .is_some_and(|selected| stream.get_device_name().as_deref() != Some(selected))
                {
                    return Err(error(
                        "Аудиосервер выбрал другой микрофон вместо указанного",
                    ));
                }
                if let Some(started) = started.take()
                    && started.send(Ok(())).is_err()
                {
                    break;
                }
                // Ограниченный объём за итерацию сохраняет отзывчивость отмены при backlog.
                for _ in 0..16 {
                    let samples = match stream.peek().map_err(error)? {
                        stream::PeekResult::Empty => break,
                        stream::PeekResult::Hole(bytes) => vec![0.0; bytes / 4],
                        stream::PeekResult::Data(data) => decode_samples(data),
                    };
                    stream.discard().map_err(error)?;
                    match pcm.try_send(super::super::pcm::Chunk {
                        samples,
                        epoch: None,
                    }) {
                        Ok(()) => backlog_warned = false,
                        Err(TrySendError::Full(_)) => {
                            if !backlog_warned {
                                warn!("linux microphone PCM queue is backing up");
                                backlog_warned = true;
                            }
                        }
                        Err(TrySendError::Disconnected(_)) => return Ok(()),
                    }
                }
            }
            _ => {}
        }
        tick(&mut mainloop, started.as_ref().map(|_| deadline))?;
    }
    let _ = stream.disconnect();
    Ok(())
}

fn check_context(context: &context::Context) -> Result<(), MicrophoneError> {
    if matches!(
        context.get_state(),
        context::State::Failed | context::State::Terminated
    ) {
        return Err(error(format!(
            "Соединение с аудиосервером потеряно: {}",
            context.errno()
        )));
    }
    Ok(())
}

fn tick(mainloop: &mut Mainloop, deadline: Option<Instant>) -> Result<(), MicrophoneError> {
    if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
        return Err(error("Аудиосервер не запустил микрофон за 8 секунд"));
    }
    match mainloop.iterate(false) {
        IterateResult::Success(_) => {}
        IterateResult::Err(error_code) => return Err(error(error_code)),
        IterateResult::Quit(_) => return Err(error("Цикл аудиосервера завершён")),
    }
    std::thread::sleep(Duration::from_millis(5));
    Ok(())
}

fn decode_samples(data: &[u8]) -> Vec<f32> {
    data.chunks_exact(4)
        .map(|bytes| f32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]).clamp(-1.0, 1.0))
        .collect()
}

fn error(detail: impl std::fmt::Display) -> MicrophoneError {
    warn!(error = %detail, "linux microphone audio server operation failed");
    MicrophoneError::new(
        "Не удалось включить выбранный микрофон. Проверьте устройство и настройки звука.",
    )
}

#[cfg(test)]
mod tests;

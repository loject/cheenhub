//! Неблокирующая передача событий tracing в файловый архив.

use std::io::{self, Write};
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc::{self, SyncSender},
};
use std::thread::{self, JoinHandle};
use tracing_subscriber::{EnvFilter, fmt::MakeWriter};

use super::storage::RotatingLog;

const QUEUE_CAPACITY: usize = 1024;
const MAX_RECORD_BYTES: usize = 64 * 1024;

/// Ограничивает постоянные логи уровнем INFO, если не задан корректный фильтр.
pub(super) fn diagnostic_log_filter(override_filter: Option<&str>) -> EnvFilter {
    match override_filter {
        None => EnvFilter::new("info"),
        Some(value) => EnvFilter::try_new(value).unwrap_or_else(|error| {
            eprintln!("Некорректный фильтр диагностических логов '{value}': {error}");
            EnvFilter::new("info")
        }),
    }
}

/// Открывает архив и запускает один поток записи без блокировок в tracing.
pub(super) fn start_archive_worker(
    directory: &Path,
    max_files: usize,
    max_bytes: u64,
) -> io::Result<(QueueMakeWriter, JoinHandle<()>)> {
    let mut archive = RotatingLog::open(directory, max_files, max_bytes)?;
    let (sender, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
    let writer = QueueMakeWriter::new(sender, Arc::new(AtomicU64::new(0)));
    let worker = thread::Builder::new()
        .name("cheenhub-log-writer".to_owned())
        .spawn(move || {
            let mut error_reported = false;
            while let Ok(record) = receiver.recv() {
                if let Err(error) = archive.append(&record).and_then(|_| archive.flush()) {
                    if !error_reported {
                        eprintln!("Не удалось сохранить диагностический лог: {error}");
                        error_reported = true;
                    }
                } else {
                    error_reported = false;
                }
            }
            let _ = archive.flush();
        })?;
    Ok((writer, worker))
}

/// Создаёт отдельный буфер для каждого форматируемого события tracing.
#[derive(Clone)]
pub(super) struct QueueMakeWriter {
    sender: SyncSender<Vec<u8>>,
    dropped: Arc<AtomicU64>,
}

impl QueueMakeWriter {
    /// Подготавливает фабрику для общей ограниченной очереди.
    pub(super) fn new(sender: SyncSender<Vec<u8>>, dropped: Arc<AtomicU64>) -> Self {
        Self { sender, dropped }
    }
}

impl<'a> MakeWriter<'a> for QueueMakeWriter {
    type Writer = QueueWriter;

    fn make_writer(&'a self) -> Self::Writer {
        QueueWriter::new(self.sender.clone(), Arc::clone(&self.dropped))
    }
}

/// Собирает одну или несколько JSONL-строк без блокировки потока событий.
pub(super) struct QueueWriter {
    sender: SyncSender<Vec<u8>>,
    dropped: Arc<AtomicU64>,
    pending: Vec<u8>,
    discarding: bool,
}

impl QueueWriter {
    /// Создаёт писатель с ограниченной очередью сообщений.
    pub(super) fn new(sender: SyncSender<Vec<u8>>, dropped: Arc<AtomicU64>) -> Self {
        Self {
            sender,
            dropped,
            pending: Vec::new(),
            discarding: false,
        }
    }
}

impl Write for QueueWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // Не удерживаем в памяти сообщение крупнее MAX_RECORD_BYTES даже до \n.
        for fragment in bytes.split_inclusive(|byte| *byte == b'\n') {
            let complete = fragment.last() == Some(&b'\n');
            if self.discarding {
                if complete {
                    self.discarding = false;
                }
                continue;
            }
            if self.pending.len().saturating_add(fragment.len()) > MAX_RECORD_BYTES {
                self.pending.clear();
                self.discarding = !complete;
                self.dropped.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            self.pending.extend_from_slice(fragment);
            if complete {
                let record = std::mem::take(&mut self.pending);
                if self.sender.try_send(record).is_err() {
                    self.dropped.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "logger/tests.rs"]
mod tests;

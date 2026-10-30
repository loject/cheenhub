//! Ротируемое файловое хранилище JSONL-событий нативного клиента.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Ограниченный архив событий: нулевой файл активен, остальные старше.
pub(super) struct RotatingLog {
    directory: PathBuf,
    max_files: usize,
    max_bytes: u64,
    active: Option<File>,
    active_bytes: u64,
}

impl RotatingLog {
    /// Открывает архив и удаляет незавершённую последнюю строку после сбоя.
    pub(super) fn open(directory: &Path, max_files: usize, max_bytes: u64) -> io::Result<Self> {
        if max_files == 0 || max_bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "archive limits must be nonzero",
            ));
        }
        fs::create_dir_all(directory)?;
        let active_path = Self::file_path(directory, 0);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&active_path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let complete_bytes = if bytes.last() == Some(&b'\n') {
            bytes.len()
        } else {
            bytes
                .iter()
                .rposition(|byte| *byte == b'\n')
                .map_or(0, |pos| pos + 1)
        };
        if complete_bytes != bytes.len() {
            file.set_len(complete_bytes as u64)?;
        }
        file.seek(SeekFrom::End(0))?;
        let mut writer = Self {
            directory: directory.to_path_buf(),
            max_files,
            max_bytes,
            active: Some(file),
            active_bytes: complete_bytes as u64,
        };
        if writer.active_bytes > max_bytes {
            // Не переносим нарушающий лимит старый активный файл в архив.
            let active = writer.active.as_mut().expect("active log is open");
            active.set_len(0)?;
            active.seek(SeekFrom::Start(0))?;
            writer.active_bytes = 0;
        }
        Ok(writer)
    }

    /// Дописывает одну завершённую JSONL-строку, вращая архив при необходимости.
    pub(super) fn append(&mut self, line: &[u8]) -> io::Result<()> {
        if line.last() != Some(&b'\n') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "JSONL entry must end with newline",
            ));
        }
        const OMITTED: &[u8] = b"{\"fields\":{\"message\":\"oversized log entry omitted\"}}\n";
        let bounded = if line.len() as u64 > self.max_bytes {
            if OMITTED.len() as u64 > self.max_bytes {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "archive byte limit is too small for a replacement record",
                ));
            }
            OMITTED
        } else {
            line
        };
        if self.active_bytes.saturating_add(bounded.len() as u64) > self.max_bytes {
            self.rotate()?;
        }
        self.active
            .as_mut()
            .expect("active log is open")
            .write_all(bounded)?;
        self.active_bytes += bounded.len() as u64;
        Ok(())
    }

    /// Сбрасывает буфер активного файла на запись.
    pub(super) fn flush(&mut self) -> io::Result<()> {
        self.active.as_mut().expect("active log is open").flush()
    }

    fn rotate(&mut self) -> io::Result<()> {
        if let Some(mut active) = self.active.take() {
            active.flush()?;
            drop(active);
        }
        for index in (1..self.max_files).rev() {
            let from = Self::file_path(&self.directory, index - 1);
            let to = Self::file_path(&self.directory, index);
            if from.exists() {
                if to.exists() {
                    fs::remove_file(&to)?;
                }
                fs::rename(from, to)?;
            }
        }
        let active_path = Self::file_path(&self.directory, 0);
        self.active = Some(
            OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(active_path)?,
        );
        self.active_bytes = 0;
        Ok(())
    }

    fn file_path(directory: &Path, index: usize) -> PathBuf {
        directory.join(format!("client.{index}.jsonl"))
    }
}

#[cfg(test)]
#[path = "storage/tests.rs"]
mod tests;

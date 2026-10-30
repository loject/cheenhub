//! Чтение и ограниченный экспорт диагностического архива.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

use chrono::DateTime;
use serde_json::Value;

const ARCHIVE_FILES: usize = 5;
const PAGE_LIMIT: usize = 500;

/// Фильтры, общие для просмотра и экспорта событий.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct LogFilter {
    pub(super) level: Option<String>,
    pub(super) target: String,
    pub(super) text: String,
    pub(super) since: Option<String>,
    pub(super) until: Option<String>,
}

/// Одна полная диагностическая запись, пригодная для копирования.
#[derive(Clone, Debug)]
pub(super) struct LogEntry {
    pub(super) timestamp: String,
    pub(super) level: String,
    pub(super) target: String,
    pub(super) message: String,
    pub(super) raw: String,
}

/// Страница событий в порядке от новых к старым.
pub(super) struct LogPage {
    pub(super) total: usize,
    pub(super) entries: Vec<LogEntry>,
}

/// Ограничения ручного экспорта событий.
#[derive(Clone, Copy)]
pub(super) struct ExportLimits {
    pub(super) max_lines: usize,
    pub(super) max_bytes: u64,
}

impl Default for ExportLimits {
    fn default() -> Self {
        Self {
            max_lines: 50_000,
            max_bytes: 10 * 1024 * 1024,
        }
    }
}

/// Результат экспорта с признаком отсечения лишних событий.
pub(super) struct ExportResult {
    pub(super) lines: usize,
    pub(super) bytes: u64,
    pub(super) truncated: bool,
}

/// Читает ограниченную страницу подходящих событий.
pub(super) fn list_logs(
    directory: &Path,
    filter: &LogFilter,
    offset: usize,
    limit: usize,
) -> io::Result<LogPage> {
    let matched = matching_logs(directory, filter)?;
    let total = matched.len();
    let entries = matched
        .into_iter()
        .skip(offset)
        .take(limit.min(PAGE_LIMIT))
        .collect();
    Ok(LogPage { total, entries })
}

/// Сохраняет только целые JSONL-записи, выбранные теми же фильтрами.
pub(super) fn export_logs(
    directory: &Path,
    output: &Path,
    filter: &LogFilter,
    limits: ExportLimits,
) -> io::Result<ExportResult> {
    // Снимок создаётся до открытия файла экспорта и не меняется при ротации.
    let matched = matching_logs(directory, filter)?;
    let mut file = File::create(output)?;
    let max_lines = limits.max_lines.min(50_000);
    let max_bytes = limits.max_bytes.min(10 * 1024 * 1024);
    let mut result = ExportResult {
        lines: 0,
        bytes: 0,
        truncated: false,
    };
    for entry in matched {
        let length = entry.raw.len() as u64;
        if result.lines >= max_lines || result.bytes.saturating_add(length) > max_bytes {
            result.truncated = true;
            break;
        }
        file.write_all(entry.raw.as_bytes())?;
        result.lines += 1;
        result.bytes += length;
    }
    file.flush()?;
    Ok(result)
}

fn matching_logs(directory: &Path, filter: &LogFilter) -> io::Result<Vec<LogEntry>> {
    let since = parse_bound(filter.since.as_deref())?;
    let until = parse_bound(filter.until.as_deref())?;
    if since.zip(until).is_some_and(|(lower, upper)| lower > upper) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid time interval",
        ));
    }
    let target = filter.target.to_lowercase();
    let text = filter.text.to_lowercase();
    let mut entries = Vec::new();
    for index in (0..ARCHIVE_FILES).rev() {
        let path = directory.join(format!("client.{index}.jsonl"));
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        // Последнюю строку без перевода строки считаем незавершённой.
        for line in bytes.split_inclusive(|byte| *byte == b'\n') {
            if line.last() != Some(&b'\n') {
                continue;
            }
            let Ok(raw) = std::str::from_utf8(line) else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<Value>(raw) else {
                continue;
            };
            let Some(entry) = parse_entry(&value, raw) else {
                continue;
            };
            if filter
                .level
                .as_ref()
                .is_some_and(|level| !entry.level.eq_ignore_ascii_case(level))
                || !entry.target.to_lowercase().contains(&target)
                || !entry.raw.to_lowercase().contains(&text)
            {
                continue;
            }
            if since.is_some() || until.is_some() {
                let Ok(timestamp) = DateTime::parse_from_rfc3339(&entry.timestamp) else {
                    continue;
                };
                if since.is_some_and(|lower| timestamp < lower)
                    || until.is_some_and(|upper| timestamp > upper)
                {
                    continue;
                }
            }
            entries.push(entry);
        }
    }
    entries.reverse();
    Ok(entries)
}

fn parse_bound(value: Option<&str>) -> io::Result<Option<DateTime<chrono::FixedOffset>>> {
    value
        .filter(|value| !value.is_empty())
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
        })
        .transpose()
}

fn parse_entry(value: &Value, raw: &str) -> Option<LogEntry> {
    let timestamp = value.get("timestamp")?.as_str()?.to_owned();
    let level = value.get("level")?.as_str()?.to_owned();
    let target = value.get("target")?.as_str()?.to_owned();
    let message = value
        .pointer("/fields/message")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| value.get("fields").map(ToString::to_string))
        .unwrap_or_default();
    Some(LogEntry {
        timestamp,
        level,
        target,
        message,
        raw: raw.to_owned(),
    })
}

#[cfg(test)]
#[path = "query/tests.rs"]
mod tests;

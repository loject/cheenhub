//! Общие Linux-счётчики CPU, памяти, накопителя и времени измерений.

use std::time::Instant;

use anyhow::Context;

/// Снимок общих счётчиков CPU и логических процессоров.
#[derive(Clone)]
pub(super) struct CpuSnapshot {
    /// Общие счётчики всех процессоров.
    pub(super) total: CpuCounters,
    /// Счётчики отдельных логических процессоров.
    pub(super) logical: Vec<CpuCounters>,
}

impl CpuSnapshot {
    /// Возвращает общую и поядерную загрузку между снимками.
    pub(super) fn usage_since(&self, previous: &Self) -> (f32, Vec<f32>) {
        let total = self.total.usage_since(&previous.total);
        let logical = self
            .logical
            .iter()
            .zip(&previous.logical)
            .map(|(current, previous)| current.usage_since(previous))
            .collect();
        (total, logical)
    }
}

/// Время CPU в USER_HZ; guest уже включён в user/nice и повторно не суммируется.
#[derive(Clone, Copy)]
pub(super) struct CpuCounters {
    /// Сумма непересекающихся состояний CPU в тиках.
    pub(super) total: u64,
    idle: u64,
}

impl CpuCounters {
    fn usage_since(self, previous: &Self) -> f32 {
        let total = self.total.saturating_sub(previous.total);
        let idle = self.idle.saturating_sub(previous.idle);
        if total == 0 {
            return 0.0;
        }
        (((total.saturating_sub(idle)) as f64 / total as f64) * 100.0) as f32
    }
}

/// Читает stat из указанного дерева procfs; ошибка означает недоступность CPU.
pub(super) async fn read_cpu_snapshot(proc_root: &std::path::Path) -> anyhow::Result<CpuSnapshot> {
    let contents = tokio::fs::read_to_string(proc_root.join("stat"))
        .await
        .context("failed to read host CPU counters")?;
    let mut counters = contents
        .lines()
        .filter(|line| line.starts_with("cpu"))
        .map(parse_cpu_line);
    let total = counters
        .next()
        .transpose()?
        .context("host CPU counters are missing")?;
    let logical = counters.collect::<anyhow::Result<Vec<_>>>()?;
    Ok(CpuSnapshot { total, logical })
}

fn parse_cpu_line(line: &str) -> anyhow::Result<CpuCounters> {
    let values = line
        .split_whitespace()
        .skip(1)
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() < 5 {
        anyhow::bail!("host CPU counter line is incomplete");
    }
    Ok(CpuCounters {
        total: values.iter().take(8).copied().sum(),
        idle: values[3].saturating_add(values[4]),
    })
}

/// Общая и используемая память Linux с учётом MemAvailable.
pub(super) struct MemorySnapshot {
    /// Полная ёмкость в байтах.
    pub(super) total_bytes: u64,
    /// Занятый объём в байтах.
    pub(super) used_bytes: u64,
}

/// Читает объём RAM и доступную память из meminfo.
pub(super) async fn read_memory_snapshot(
    proc_root: &std::path::Path,
) -> anyhow::Result<MemorySnapshot> {
    let contents = tokio::fs::read_to_string(proc_root.join("meminfo"))
        .await
        .context("failed to read host memory counters")?;
    let total_kib = meminfo_value(&contents, "MemTotal")?;
    let available_kib = meminfo_value(&contents, "MemAvailable")?;
    let total_bytes = total_kib.saturating_mul(1024);
    Ok(MemorySnapshot {
        total_bytes,
        used_bytes: total_bytes.saturating_sub(available_kib.saturating_mul(1024)),
    })
}

fn meminfo_value(contents: &str, key: &str) -> anyhow::Result<u64> {
    contents
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            (name == key).then(|| value.split_whitespace().next()?.parse().ok())?
        })
        .with_context(|| format!("host memory counter {key} is missing"))
}

/// Использование файловой системы, содержащей выбранный путь.
pub(super) struct DiskSnapshot {
    /// Полная ёмкость в байтах.
    pub(super) total_bytes: u64,
    /// Занятый объём в байтах.
    pub(super) used_bytes: u64,
}

/// Читает статистику накопителя; `None`, если путь недоступен или statvfs не удался.
pub(super) fn read_disk_snapshot(path: &str) -> Option<DiskSnapshot> {
    let c_path = std::ffi::CString::new(path).ok()?;
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c_path.as_ptr(), &mut stats) } != 0 {
        tracing::warn!(
            disk_path = %path,
            error = %std::io::Error::last_os_error(),
            "host disk metrics are unavailable"
        );
        return None;
    }
    Some(disk_snapshot_from_statvfs(&stats))
}

/// Считает занятое место как total - free. `f_bfree` учитывает все свободные блоки,
/// тогда как `f_bavail` дополнительно исключает зарезервированные для root.
fn disk_snapshot_from_statvfs(stats: &libc::statvfs) -> DiskSnapshot {
    let block_bytes = stats.f_frsize;
    let total_bytes = stats.f_blocks.saturating_mul(block_bytes);
    let free_bytes = stats.f_bfree.saturating_mul(block_bytes);
    DiskSnapshot {
        total_bytes,
        used_bytes: total_bytes.saturating_sub(free_bytes),
    }
}

/// Базовый сетевой снимок для расчёта скорости между измерениями.
pub(super) struct NetworkSnapshot {
    /// Монотонное время измерения.
    pub(super) sampled_at: Instant,
    /// Накопленные исходящие байты.
    pub(super) sent_bytes: u64,
    /// Накопленные входящие байты.
    pub(super) received_bytes: u64,
}

/// Возвращает Unix-время в миллисекундах с насыщением диапазона i64.
pub(super) fn unix_timestamp_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

#[cfg(test)]
mod tests;

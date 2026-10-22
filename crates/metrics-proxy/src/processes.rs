//! Чтение Linux-процессов backend и локального PostgreSQL без запуска внешних команд.

use std::{collections::HashMap, path::Path};

use anyhow::Context;

/// Нагрузка процесса с идентификатором запуска для защиты от повторного использования PID.
pub(super) struct ProcessSnapshot {
    /// Время запуска в тиках относительно старта системы.
    pub(super) started_at: u64,
    /// Сумма utime и stime процесса в USER_HZ, без повторного учёта детей.
    pub(super) ticks: u64,
    /// Приватная резидентная память в байтах: resident минус shared из statm.
    pub(super) private_bytes: u64,
    /// Процесс относится к PostgreSQL; остальные выбранные процессы принадлежат backend.
    pub(super) database: bool,
}

/// Читает доступные процессы; завершившиеся и недоступные PID пропускает с диагностикой.
///
/// PostgreSQL учитывается целиком на локальном хосте: ОС не разделяет нагрузку
/// одного кластера между базами данных. Удалённая БД в эти счётчики не входит.
pub(super) fn read_processes(
    proc_root: &Path,
    backend_pid: u32,
) -> anyhow::Result<HashMap<u32, ProcessSnapshot>> {
    let page_bytes = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    let page_bytes = u64::try_from(page_bytes).context("failed to read system page size")?;
    let mut snapshots = HashMap::new();
    for entry in std::fs::read_dir(proc_root).context("failed to list host processes")? {
        let entry = entry?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        let root = entry.path();
        let result = (|| -> anyhow::Result<Option<ProcessSnapshot>> {
            let stat = std::fs::read_to_string(root.join("stat"))?;
            let (name, ticks, started_at) = parse_stat(&stat)?;
            let database = matches!(name, "postgres" | "postmaster");
            if pid != backend_pid && !database {
                return Ok(None);
            }
            let statm = std::fs::read_to_string(root.join("statm"))?;
            let private_bytes = private_memory(&statm, page_bytes)?;
            Ok(Some(ProcessSnapshot {
                started_at,
                ticks,
                private_bytes,
                database,
            }))
        })();
        match result {
            Ok(Some(snapshot)) => {
                snapshots.insert(pid, snapshot);
            }
            Ok(None) => {}
            Err(error) => {
                tracing::debug!(pid, %error, "process metrics unavailable; process may have exited or access is restricted")
            }
        }
    }
    Ok(snapshots)
}

fn parse_stat(stat: &str) -> anyhow::Result<(&str, u64, u64)> {
    let (_, tail) = stat.split_once('(').context("process stat has no name")?;
    let (name, fields) = tail
        .rsplit_once(')')
        .context("process stat has no closing name delimiter")?;
    let fields: Vec<_> = fields.split_whitespace().collect();
    if fields.len() < 20 {
        anyhow::bail!("process stat is incomplete");
    }
    let ticks = fields[11]
        .parse::<u64>()?
        .saturating_add(fields[12].parse()?);
    Ok((name, ticks, fields[19].parse()?))
}

fn private_memory(statm: &str, page_bytes: u64) -> anyhow::Result<u64> {
    let fields = statm
        .split_whitespace()
        .take(3)
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()?;
    if fields.len() < 3 {
        anyhow::bail!("process statm is incomplete");
    }
    Ok(fields[1]
        .saturating_sub(fields[2])
        .saturating_mul(page_bytes))
}

#[cfg(test)]
mod tests;

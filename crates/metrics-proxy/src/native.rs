//! Прямой сбор метрик Linux-хоста для backend, работающего вне контейнера.

use std::{collections::HashMap, path::PathBuf, time::Instant};

use anyhow::Context;
use cheenhub_contracts::rest::{
    HostCpuMetrics, HostDiskMetrics, HostMemoryMetrics, HostMetricsSample, HostNetworkMetrics,
    HostNetworkScope,
};

use super::{
    processes::{ProcessSnapshot, read_processes},
    system::{
        CpuSnapshot, read_cpu_snapshot, read_disk_snapshot, read_memory_snapshot,
        unix_timestamp_millis,
    },
};

/// Собирает CPU, RAM, накопитель и сетевой трафик хоста без Docker и отдельного proxy.
///
/// Доли процессов нормированы к мощности всего хоста. Память процессов исключает
/// shared-страницы, чтобы не умножать общую память PostgreSQL на число его workers.
pub struct NativeMetricsCollector {
    proc_root: PathBuf,
    disk_path: String,
    backend_pid: u32,
    previous: Option<Snapshot>,
}

struct Snapshot {
    cpu: CpuSnapshot,
    processes: HashMap<u32, ProcessSnapshot>,
    network: HashMap<String, (u64, u64)>,
    sampled_at: Instant,
}

impl NativeMetricsCollector {
    /// Создаёт сборщик текущего backend и файловой системы указанного пути.
    ///
    /// Обычно путь равен `/`; отдельный накопитель можно выбрать через конфигурацию backend.
    pub fn new(disk_path: String) -> Self {
        Self {
            proc_root: PathBuf::from("/proc"),
            disk_path,
            backend_pid: std::process::id(),
            previous: None,
        }
    }

    /// Возвращает снимок после базового замера, необходимого для расчёта загрузки и скоростей.
    ///
    /// # Errors
    /// Возвращает ошибку при недоступности CPU, RAM, сети или списка процессов.
    /// Недоступный накопитель исключается из снимка, не прерывая остальные измерения.
    pub async fn collect(&mut self) -> anyhow::Result<Option<HostMetricsSample>> {
        let cpu = read_cpu_snapshot(&self.proc_root).await?;
        let memory = read_memory_snapshot(&self.proc_root).await?;
        let network =
            read_network(&tokio::fs::read_to_string(self.proc_root.join("net/dev")).await?)?;
        let root = self.proc_root.clone();
        let pid = self.backend_pid;
        let disk_path = self.disk_path.clone();
        let (processes, disk) = tokio::task::spawn_blocking(move || {
            Ok::<_, anyhow::Error>((read_processes(&root, pid)?, read_disk_snapshot(&disk_path)))
        })
        .await
        .context("host metrics reader failed")??;
        let current = Snapshot {
            cpu,
            processes,
            network,
            sampled_at: Instant::now(),
        };
        let previous = self.previous.replace(current);
        let current = self.previous.as_ref().expect("current snapshot was stored");
        let Some(previous) = previous else {
            return Ok(None);
        };
        let (system_percent, logical_processors_percent) = current.cpu.usage_since(&previous.cpu);
        let total_delta = current
            .cpu
            .total
            .total
            .saturating_sub(previous.cpu.total.total);
        let (app_cpu, database_cpu, app_memory, database_memory) =
            process_usage(current, &previous, total_delta);
        let app_cpu = app_cpu.clamp(0.0, system_percent);
        let database_cpu = database_cpu.clamp(0.0, (system_percent - app_cpu).max(0.0));
        let elapsed = current
            .sampled_at
            .saturating_duration_since(previous.sampled_at)
            .as_secs_f64()
            .max(0.001);
        Ok(Some(HostMetricsSample {
            sampled_at_unix_ms: unix_timestamp_millis(),
            cpu: HostCpuMetrics {
                system_percent,
                cheenhub_percent: app_cpu,
                database_percent: database_cpu,
                other_percent: (system_percent - app_cpu - database_cpu).max(0.0),
                logical_processors_percent,
            },
            memory: HostMemoryMetrics {
                total_bytes: memory.total_bytes,
                used_bytes: memory.used_bytes,
                cheenhub_bytes: app_memory,
                database_bytes: database_memory,
                other_bytes: memory
                    .used_bytes
                    .saturating_sub(app_memory)
                    .saturating_sub(database_memory),
            },
            disk: disk.map(|disk| HostDiskMetrics {
                total_bytes: disk.total_bytes,
                used_bytes: disk.used_bytes,
            }),
            network: network_metrics(&current.network, &previous.network, elapsed),
        }))
    }
}

fn process_usage(
    current: &Snapshot,
    previous: &Snapshot,
    total_delta: u64,
) -> (f32, f32, u64, u64) {
    let mut app_ticks = 0_u64;
    let mut database_ticks = 0_u64;
    let mut app_memory = 0_u64;
    let mut database_memory = 0_u64;
    for (pid, process) in &current.processes {
        let ticks = previous
            .processes
            .get(pid)
            .filter(|old| old.started_at == process.started_at && old.database == process.database)
            .map_or(0, |old| process.ticks.saturating_sub(old.ticks));
        if process.database {
            database_ticks = database_ticks.saturating_add(ticks);
            database_memory = database_memory.saturating_add(process.private_bytes);
        } else {
            app_ticks = app_ticks.saturating_add(ticks);
            app_memory = app_memory.saturating_add(process.private_bytes);
        }
    }
    let share = |ticks| {
        if total_delta == 0 {
            0.0
        } else {
            (ticks as f64 / total_delta as f64 * 100.0) as f32
        }
    };
    (
        share(app_ticks),
        share(database_ticks),
        app_memory,
        database_memory,
    )
}

fn network_metrics(
    current: &HashMap<String, (u64, u64)>,
    previous: &HashMap<String, (u64, u64)>,
    elapsed: f64,
) -> HostNetworkMetrics {
    let mut sent_total = 0_u64;
    let mut received_total = 0_u64;
    let mut sent_delta = 0_u64;
    let mut received_delta = 0_u64;
    for (name, &(received, sent)) in current {
        sent_total = sent_total.saturating_add(sent);
        received_total = received_total.saturating_add(received);
        if let Some(&(previous_received, previous_sent)) = previous.get(name) {
            sent_delta = sent_delta.saturating_add(sent.saturating_sub(previous_sent));
            received_delta =
                received_delta.saturating_add(received.saturating_sub(previous_received));
        }
    }
    HostNetworkMetrics {
        scope: HostNetworkScope::Host,
        sent_bytes_per_second: sent_delta as f64 / elapsed,
        received_bytes_per_second: received_delta as f64 / elapsed,
        sent_bytes_total: sent_total,
        received_bytes_total: received_total,
    }
}

fn read_network(contents: &str) -> anyhow::Result<HashMap<String, (u64, u64)>> {
    let mut interfaces = HashMap::new();
    for line in contents.lines() {
        let Some((name, values)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if name == "lo" {
            continue;
        }
        let fields: Vec<_> = values.split_whitespace().collect();
        if fields.len() < 9 {
            anyhow::bail!("host network counters are incomplete for {name}");
        }
        interfaces.insert(name.to_owned(), (fields[0].parse()?, fields[8].parse()?));
    }
    Ok(interfaces)
}

#[cfg(test)]
mod tests;

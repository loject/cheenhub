//! Сбор и агрегация разрешённых системных метрик.

use std::{collections::HashSet, time::Instant};

use anyhow::Context;
use cheenhub_contracts::rest::{
    HostCpuMetrics, HostDiskMetrics, HostMemoryMetrics, HostMetricsSample, HostNetworkMetrics,
};

use super::docker::{ContainerStats, DockerClient};

const COMPOSE_SERVICE_LABEL: &str = "com.docker.compose.service";

pub(super) struct MetricsCollector {
    docker: DockerClient,
    app_services: HashSet<String>,
    database_service: String,
    disk_path: String,
    previous_cpu: Option<CpuSnapshot>,
    previous_network: Option<NetworkSnapshot>,
}

impl MetricsCollector {
    pub(super) fn new(
        socket_path: String,
        app_services: Vec<String>,
        database_service: String,
        disk_path: String,
    ) -> Self {
        Self {
            docker: DockerClient::new(socket_path),
            app_services: app_services.into_iter().collect(),
            database_service,
            disk_path,
            previous_cpu: None,
            previous_network: None,
        }
    }

    pub(super) async fn collect(&mut self) -> anyhow::Result<Option<HostMetricsSample>> {
        let cpu = read_cpu_snapshot().await?;
        let memory = read_memory_snapshot().await?;
        // Ошибка диска не должна ломать сбор CPU/RAM/network: отдаём sample без disk.
        let disk = read_disk_snapshot(&self.disk_path);
        let containers = self.docker.running_containers().await?;
        let mut app_stats = Vec::new();
        let mut database_stats = Vec::new();

        for container in containers {
            let Some(service) = container.labels.get(COMPOSE_SERVICE_LABEL) else {
                continue;
            };
            if self.app_services.contains(service) {
                app_stats.push(self.docker.container_stats(&container.id).await?);
            } else if service == &self.database_service {
                database_stats.push(self.docker.container_stats(&container.id).await?);
            }
        }
        if app_stats.is_empty() {
            anyhow::bail!("CheenHub application containers were not found");
        }
        if database_stats.is_empty() {
            anyhow::bail!("CheenHub database container was not found");
        }

        let previous_cpu = self.previous_cpu.replace(cpu.clone());
        let network_totals = aggregate_network(&app_stats);
        let now = Instant::now();
        let previous_network = self.previous_network.replace(NetworkSnapshot {
            sampled_at: now,
            sent_bytes: network_totals.0,
            received_bytes: network_totals.1,
        });
        let (Some(previous_cpu), Some(previous_network)) = (previous_cpu, previous_network) else {
            return Ok(None);
        };

        let (system_percent, logical_processors_percent) = cpu.usage_since(&previous_cpu);
        let cheenhub_cpu = aggregate_cpu(&app_stats).clamp(0.0, 100.0);
        let database_cpu = aggregate_cpu(&database_stats).clamp(0.0, 100.0);
        let other_cpu = (system_percent - cheenhub_cpu - database_cpu).max(0.0);
        let cheenhub_memory = aggregate_memory(&app_stats);
        let database_memory = aggregate_memory(&database_stats);
        let other_memory = memory
            .used_bytes
            .saturating_sub(cheenhub_memory)
            .saturating_sub(database_memory);
        let elapsed = now
            .saturating_duration_since(previous_network.sampled_at)
            .as_secs_f64()
            .max(0.001);

        Ok(Some(HostMetricsSample {
            sampled_at_unix_ms: unix_timestamp_millis(),
            cpu: HostCpuMetrics {
                system_percent,
                cheenhub_percent: cheenhub_cpu,
                database_percent: database_cpu,
                other_percent: other_cpu,
                logical_processors_percent,
            },
            memory: HostMemoryMetrics {
                total_bytes: memory.total_bytes,
                used_bytes: memory.used_bytes,
                cheenhub_bytes: cheenhub_memory,
                database_bytes: database_memory,
                other_bytes: other_memory,
            },
            disk: disk.map(|disk| HostDiskMetrics {
                total_bytes: disk.total_bytes,
                used_bytes: disk.used_bytes,
            }),
            network: HostNetworkMetrics {
                sent_bytes_per_second: network_totals.0.saturating_sub(previous_network.sent_bytes)
                    as f64
                    / elapsed,
                received_bytes_per_second: network_totals
                    .1
                    .saturating_sub(previous_network.received_bytes)
                    as f64
                    / elapsed,
                sent_bytes_total: network_totals.0,
                received_bytes_total: network_totals.1,
            },
        }))
    }
}

fn aggregate_cpu(stats: &[ContainerStats]) -> f32 {
    stats
        .iter()
        .map(ContainerStats::whole_host_cpu_percent)
        .sum()
}

fn aggregate_memory(stats: &[ContainerStats]) -> u64 {
    stats
        .iter()
        .map(ContainerStats::effective_memory_bytes)
        .fold(0, u64::saturating_add)
}

fn aggregate_network(stats: &[ContainerStats]) -> (u64, u64) {
    stats.iter().fold((0, 0), |(sent, received), stats| {
        let network = stats.network_totals();
        (
            sent.saturating_add(network.0),
            received.saturating_add(network.1),
        )
    })
}

#[derive(Clone)]
struct CpuSnapshot {
    total: CpuCounters,
    logical: Vec<CpuCounters>,
}

impl CpuSnapshot {
    fn usage_since(&self, previous: &Self) -> (f32, Vec<f32>) {
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

#[derive(Clone, Copy)]
struct CpuCounters {
    total: u64,
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

async fn read_cpu_snapshot() -> anyhow::Result<CpuSnapshot> {
    let contents = tokio::fs::read_to_string("/proc/stat")
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
        total: values.iter().copied().sum(),
        idle: values[3].saturating_add(values[4]),
    })
}

struct MemorySnapshot {
    total_bytes: u64,
    used_bytes: u64,
}

async fn read_memory_snapshot() -> anyhow::Result<MemorySnapshot> {
    let contents = tokio::fs::read_to_string("/proc/meminfo")
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

struct DiskSnapshot {
    total_bytes: u64,
    used_bytes: u64,
}

/// Читает статистику накопителя; `None`, если путь недоступен или statvfs не удался.
fn read_disk_snapshot(path: &str) -> Option<DiskSnapshot> {
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

struct NetworkSnapshot {
    sampled_at: Instant,
    sent_bytes: u64,
    received_bytes: u64,
}

fn unix_timestamp_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

#[cfg(test)]
mod tests {
    use super::{
        CpuCounters, disk_snapshot_from_statvfs, meminfo_value, parse_cpu_line, read_disk_snapshot,
    };

    #[test]
    fn calculates_cpu_usage_from_counter_delta() {
        let previous = CpuCounters {
            total: 100,
            idle: 40,
        };
        let current = CpuCounters {
            total: 200,
            idle: 65,
        };
        assert_eq!(current.usage_since(&previous), 75.0);
    }

    #[test]
    fn parses_linux_cpu_line() {
        let counters = parse_cpu_line("cpu0 10 2 3 40 5 1 2 0 0 0").expect("line parses");
        assert_eq!(counters.total, 63);
        assert_eq!(counters.idle, 45);
    }

    #[test]
    fn reads_memory_value_in_kibibytes() {
        assert_eq!(
            meminfo_value("MemTotal:       16384 kB\n", "MemTotal").expect("value exists"),
            16_384
        );
    }

    #[test]
    fn calculates_disk_usage_from_free_blocks_not_available() {
        let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
        stats.f_frsize = 4096;
        stats.f_blocks = 1000;
        stats.f_bfree = 400;
        stats.f_bavail = 350;

        let snapshot = disk_snapshot_from_statvfs(&stats);

        assert_eq!(snapshot.total_bytes, 1000 * 4096);
        // Занятое место считается от f_bfree, а не от f_bavail:
        // зарезервированные блоки для root не являются занятыми.
        assert_eq!(snapshot.used_bytes, (1000 - 400) * 4096);
    }

    #[test]
    fn reads_disk_snapshot_from_temporary_directory() {
        let directory =
            std::env::temp_dir().join(format!("cheenhub-disk-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("temporary directory is created");
        let snapshot = read_disk_snapshot(directory.to_str().expect("path is valid UTF-8"))
            .expect("disk snapshot reads");
        assert!(snapshot.total_bytes > 0);
        assert!(snapshot.used_bytes <= snapshot.total_bytes);
    }

    #[test]
    fn returns_none_when_disk_path_is_missing() {
        let missing =
            std::env::temp_dir().join(format!("cheenhub-disk-missing-{}", std::process::id()));
        assert!(read_disk_snapshot(missing.to_str().expect("path is valid UTF-8")).is_none());
    }
}

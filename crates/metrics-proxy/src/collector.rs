//! Сбор и агрегация разрешённых системных метрик.

use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use cheenhub_contracts::rest::{
    HostCpuMetrics, HostDiskMetrics, HostMemoryMetrics, HostMetricsSample, HostNetworkMetrics,
    HostNetworkScope,
};

use super::docker::{ContainerStats, CpuStats, DockerClient};
use super::system::{
    CpuSnapshot, NetworkSnapshot, read_cpu_snapshot, read_disk_snapshot, read_memory_snapshot,
    unix_timestamp_millis,
};

const COMPOSE_SERVICE_LABEL: &str = "com.docker.compose.service";

/// Собирает нагрузку хоста и выбранных Docker Compose сервисов через Docker socket.
pub struct MetricsCollector {
    docker: DockerClient,
    app_services: HashSet<String>,
    database_service: String,
    disk_path: String,
    previous_cpu: Option<CpuSnapshot>,
    previous_container_cpu: HashMap<String, CpuStats>,
    previous_network: Option<NetworkSnapshot>,
}

impl MetricsCollector {
    /// Создаёт сборщик с путём socket, именами Compose сервисов и путём файловой системы.
    pub fn new(
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
            previous_container_cpu: HashMap::new(),
            previous_network: None,
        }
    }

    /// Возвращает снимок после базового замера или ошибку чтения системных/Docker счётчиков.
    ///
    /// # Errors
    /// Ошибка означает недоступность CPU/RAM, Docker API либо отсутствие выбранных сервисов.
    pub async fn collect(&mut self) -> anyhow::Result<Option<HostMetricsSample>> {
        let cpu = read_cpu_snapshot(std::path::Path::new("/proc")).await?;
        let memory = read_memory_snapshot(std::path::Path::new("/proc")).await?;
        // Ошибка диска не должна ломать сбор CPU/RAM/network: отдаём sample без disk.
        let disk = read_disk_snapshot(&self.disk_path);
        let containers = self.docker.running_containers().await?;
        let mut app_stats = Vec::new();
        let mut database_stats = Vec::new();
        let mut next_container_cpu = HashMap::new();
        let mut cheenhub_cpu = 0.0_f32;
        let mut database_cpu = 0.0_f32;

        for container in containers {
            let Some(service) = container.labels.get(COMPOSE_SERVICE_LABEL) else {
                continue;
            };
            let is_app = self.app_services.contains(service);
            let is_database = service == &self.database_service;
            if !is_app && !is_database {
                continue;
            }

            let stats = self.docker.container_stats(&container.id).await?;
            if let Some(previous) = self.previous_container_cpu.get(&container.id) {
                if let Some(percent) = stats.cpu_stats.whole_host_cpu_percent_since(*previous) {
                    if is_app {
                        cheenhub_cpu += percent;
                    } else {
                        database_cpu += percent;
                    }
                } else {
                    tracing::debug!(
                        container_id = %container.id,
                        service = %service,
                        "Docker CPU counters did not advance or were reset"
                    );
                }
            } else {
                tracing::debug!(
                    container_id = %container.id,
                    service = %service,
                    "initializing Docker container CPU baseline"
                );
            }
            next_container_cpu.insert(container.id, stats.cpu_stats);
            if is_app {
                app_stats.push(stats);
            } else {
                database_stats.push(stats);
            }
        }
        if app_stats.is_empty() {
            anyhow::bail!("CheenHub application containers were not found");
        }
        if database_stats.is_empty() {
            anyhow::bail!("CheenHub database container was not found");
        }

        // Исключаем остановленные контейнеры из истории; новые получат базовый замер.
        self.previous_container_cpu = next_container_cpu;
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
        let cheenhub_cpu = cheenhub_cpu.clamp(0.0, 100.0);
        let database_cpu = database_cpu.clamp(0.0, 100.0);
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
                scope: HostNetworkScope::Application,
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

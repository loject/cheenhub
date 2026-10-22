use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

#[test]
fn calculates_network_rates_over_controlled_interval_without_loopback_or_new_interface_spikes() {
    let previous =
        read_network("lo: 999 0 0 0 0 0 0 0 999\neth0: 100 0 0 0 0 0 0 0 200\n").unwrap();
    let current = read_network("lo: 99999 0 0 0 0 0 0 0 99999\neth0: 150 0 0 0 0 0 0 0 300\neth1: 10000 0 0 0 0 0 0 0 20000\n").unwrap();

    let network = network_metrics(&current, &previous, 2.0);

    assert_eq!(network.received_bytes_per_second, 25.0);
    assert_eq!(network.sent_bytes_per_second, 50.0);
    assert_eq!(network.received_bytes_total, 10150);
    assert_eq!(network.sent_bytes_total, 20300);
}

struct HostFixture(PathBuf);

impl HostFixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "cheenhub-native-metrics-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("net")).unwrap();
        std::fs::write(
            root.join("meminfo"),
            "MemTotal: 10000 kB\nMemAvailable: 4000 kB\n",
        )
        .unwrap();
        Self(root)
    }

    fn counters(&self, cpu: &str, network: &str) {
        std::fs::write(self.0.join("stat"), cpu).unwrap();
        std::fs::write(self.0.join("net/dev"), network).unwrap();
    }

    fn process(&self, pid: u32, name: &str, ticks: u64, start: u64) {
        let root = self.0.join(pid.to_string());
        std::fs::create_dir_all(&root).unwrap();
        let mut fields = vec!["0".to_owned(); 22];
        fields[0] = "S".to_owned();
        fields[11] = ticks.to_string();
        fields[19] = start.to_string();
        std::fs::write(
            root.join("stat"),
            format!("{pid} ({name}) {}", fields.join(" ")),
        )
        .unwrap();
        std::fs::write(root.join("statm"), "100 20 5 0 0 0 0").unwrap();
    }

    fn collector(&self) -> NativeMetricsCollector {
        let mut collector = NativeMetricsCollector::new(self.0.to_string_lossy().into_owned());
        collector.proc_root = self.0.clone();
        collector.backend_pid = 42;
        collector
    }
}

impl Drop for HostFixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[tokio::test]
async fn collects_host_resources_and_process_shares_without_docker() {
    let host = HostFixture::new();
    host.counters(
        "cpu 100 0 0 100 0\ncpu0 100 0 0 100 0\n",
        "eth0: 100 0 0 0 0 0 0 0 200 0 0 0 0 0 0 0\nlo: 999 0 0 0 0 0 0 0 999 0 0 0 0 0 0 0\n",
    );
    host.process(42, "backend", 10, 1);
    host.process(50, "postgres", 20, 2);
    host.process(70, "other", 30, 3);
    let mut collector = host.collector();
    assert!(collector.collect().await.unwrap().is_none());

    host.counters(
        "cpu 170 0 0 130 0\ncpu0 170 0 0 130 0\n",
        "eth0: 150 0 0 0 0 0 0 0 300 0 0 0 0 0 0 0\n",
    );
    host.process(42, "backend", 20, 1);
    host.process(50, "postgres", 40, 2);
    let sample = collector.collect().await.unwrap().unwrap();

    assert_eq!(sample.cpu.system_percent, 70.0);
    assert_eq!(sample.cpu.cheenhub_percent, 10.0);
    assert_eq!(sample.cpu.database_percent, 20.0);
    assert_eq!(sample.cpu.other_percent, 40.0);
    assert_eq!(sample.cpu.logical_processors_percent, vec![70.0]);
    assert_eq!(sample.memory.total_bytes, 10_240_000);
    assert_eq!(sample.memory.used_bytes, 6_144_000);
    assert!(sample.memory.cheenhub_bytes > 0);
    assert_eq!(sample.memory.cheenhub_bytes, sample.memory.database_bytes);
    assert_eq!(sample.network.scope, HostNetworkScope::Host);
    assert_eq!(sample.network.sent_bytes_total, 300);
    assert_eq!(sample.network.received_bytes_total, 150);
    assert!(sample.disk.is_some());
}

#[tokio::test]
async fn ignores_reused_process_ids_and_missing_database() {
    let host = HostFixture::new();
    host.counters("cpu 100 0 0 100 0\n", "eth0: 100 0 0 0 0 0 0 0 200\n");
    host.process(42, "backend", 10, 1);
    let mut collector = host.collector();
    collector.collect().await.unwrap();

    host.counters("cpu 170 0 0 130 0\n", "eth0: 10 0 0 0 0 0 0 0 20\n");
    host.process(42, "backend", 100, 2);
    let sample = collector.collect().await.unwrap().unwrap();

    assert_eq!(sample.cpu.cheenhub_percent, 0.0);
    assert_eq!(sample.cpu.database_percent, 0.0);
    assert_eq!(sample.memory.database_bytes, 0);
    assert_eq!(sample.network.sent_bytes_per_second, 0.0);
    assert_eq!(sample.network.received_bytes_per_second, 0.0);
}

#[tokio::test]
async fn missing_disk_keeps_other_resources_available() {
    let host = HostFixture::new();
    host.counters("cpu 100 0 0 100 0\n", "eth0: 100 0 0 0 0 0 0 0 200\n");
    let mut collector = host.collector();
    collector.disk_path = host.0.join("missing").to_string_lossy().into_owned();
    collector.collect().await.unwrap();

    let sample = collector.collect().await.unwrap().unwrap();

    assert!(sample.disk.is_none());
    assert_eq!(sample.memory.total_bytes, 10_240_000);
}

#[tokio::test]
async fn missing_cpu_counters_return_error_instead_of_zero_metrics() {
    let host = HostFixture::new();
    let mut collector = host.collector();

    assert!(collector.collect().await.is_err());
}

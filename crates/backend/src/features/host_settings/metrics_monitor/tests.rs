use super::*;

#[test]
fn host_uses_direct_collection_even_with_leftover_proxy_configuration() {
    assert_eq!(select_source(false, None), MetricsSource::Native);
    assert_eq!(
        select_source(false, Some("http://old-proxy".to_owned())),
        MetricsSource::Native
    );
}

#[test]
fn container_preserves_isolated_proxy_collection() {
    assert_eq!(
        select_source(true, Some("http://metrics-proxy:9100".to_owned())),
        MetricsSource::Proxy("http://metrics-proxy:9100".to_owned())
    );
    assert_eq!(select_source(true, None), MetricsSource::Unavailable);
}

#[tokio::test]
#[ignore = "Требуется Linux-хост вне контейнера с доступными /proc и корневой файловой системой"]
async fn collects_host_metrics_without_proxy() {
    assert!(
        !running_in_container(),
        "smoke test requires a non-container host"
    );
    let monitor = Arc::new(HostMetricsMonitor::new(None));
    let task = tokio::spawn(monitor.clone().run());

    tokio::time::sleep(Duration::from_millis(2300)).await;
    let snapshot = monitor.snapshot().await;
    task.abort();

    assert!(snapshot.available, "native host metrics must be available");
    let sample = snapshot.samples.last().expect("second measurement exists");
    assert!(sample.memory.total_bytes > 0);
    assert!(sample.disk.as_ref().expect("root disk exists").total_bytes > 0);
    assert!(!sample.cpu.logical_processors_percent.is_empty());
}

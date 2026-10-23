#![cfg(not(target_os = "linux"))]
//! Проверка публичного контракта сборщиков на неподдерживаемой ОС.

use cheenhub_metrics_proxy::{DockerMetricsCollector, NativeMetricsCollector};

#[tokio::test]
async fn native_collection_reports_unsupported_platform() {
    let mut collector = NativeMetricsCollector::new("/".to_owned());

    let error = collector.collect().await.expect_err("collection must fail");

    assert_eq!(
        error.to_string(),
        "native host metrics are supported only on Linux"
    );
}

#[tokio::test]
async fn docker_collection_reports_unsupported_platform() {
    let mut collector = DockerMetricsCollector::new(
        "/var/run/docker.sock".to_owned(),
        vec!["backend".to_owned()],
        "db".to_owned(),
        "/".to_owned(),
    );

    let error = collector.collect().await.expect_err("collection must fail");

    assert_eq!(
        error.to_string(),
        "Docker host metrics are supported only on Linux"
    );
}

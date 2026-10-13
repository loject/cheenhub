use super::{CpuStats, CpuUsage, decode_chunked};

#[test]
fn decodes_chunked_http_body() {
    let decoded = decode_chunked(b"4\r\ntest\r\n3\r\n123\r\n0\r\n\r\n").expect("body decodes");
    assert_eq!(decoded, b"test123");
}

fn cpu_stats(container_usage: u64, system_usage: u64) -> CpuStats {
    CpuStats {
        cpu_usage: CpuUsage {
            total_usage: container_usage,
        },
        system_cpu_usage: system_usage,
    }
}

#[test]
fn calculates_container_cpu_share_from_consecutive_one_shot_samples() {
    let previous = cpu_stats(40, 10_000);
    let current = cpu_stats(90, 10_100);

    assert_eq!(current.whole_host_cpu_percent_since(previous), Some(50.0));
}

#[test]
fn skips_container_cpu_share_without_host_counter_progress() {
    let previous = cpu_stats(40, 10_000);
    let current = cpu_stats(90, 10_000);

    assert_eq!(current.whole_host_cpu_percent_since(previous), None);
}

#[test]
fn skips_container_cpu_share_after_counter_reset() {
    let previous = cpu_stats(40, 10_000);

    assert_eq!(
        cpu_stats(20, 10_100).whole_host_cpu_percent_since(previous),
        None
    );
    assert_eq!(
        cpu_stats(50, 9_000).whole_host_cpu_percent_since(previous),
        None
    );
}

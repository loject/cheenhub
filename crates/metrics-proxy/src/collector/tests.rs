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
    let directory = std::env::temp_dir().join(format!("cheenhub-disk-test-{}", std::process::id()));
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

use super::*;
use std::path::PathBuf;

fn test_directory() -> PathBuf {
    let path = std::env::temp_dir().join(format!("cheenhub-query-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn event(timestamp: &str, level: &str, target: &str, message: &str) -> String {
    serde_json::json!({
        "timestamp": timestamp,
        "level": level,
        "target": target,
        "fields": { "message": message },
    })
    .to_string()
        + "\n"
}

#[test]
fn query_filters_level_target_text_and_time() {
    let directory = test_directory();
    let rows = [
        event(
            "2026-10-30T09:00:00Z",
            "WARN",
            "voice::capture",
            "late packet",
        ),
        event(
            "2026-10-30T10:15:00Z",
            "INFO",
            "voice::capture",
            "late packet",
        ),
        event(
            "2026-10-30T10:30:00Z",
            "WARN",
            "social::list",
            "late packet",
        ),
        event(
            "2026-10-30T10:45:00Z",
            "WARN",
            "voice::capture",
            "late packet",
        ),
        event("2026-10-30T11:15:00Z", "WARN", "voice::capture", "normal"),
    ];
    std::fs::write(directory.join("client.0.jsonl"), rows.concat()).unwrap();
    let filter = LogFilter {
        level: Some("WARN".to_owned()),
        target: "voice".to_owned(),
        text: "LATE".to_owned(),
        since: Some("2026-10-30T10:00:00Z".to_owned()),
        until: Some("2026-10-30T11:00:00Z".to_owned()),
    };

    let page = list_logs(&directory, &filter, 0, 20).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.entries[0].timestamp, "2026-10-30T10:45:00Z");
    assert_eq!(page.entries[0].message, "late packet");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn query_pages_newest_first_across_rotations() {
    let directory = test_directory();
    std::fs::write(
        directory.join("client.1.jsonl"),
        event("2026-10-30T10:00:00Z", "INFO", "old", "first"),
    )
    .unwrap();
    std::fs::write(
        directory.join("client.0.jsonl"),
        [
            event("2026-10-30T10:01:00Z", "INFO", "new", "second"),
            event("2026-10-30T10:02:00Z", "ERROR", "new", "third"),
        ]
        .concat(),
    )
    .unwrap();
    let page = list_logs(&directory, &LogFilter::default(), 1, 1).unwrap();
    assert_eq!(page.total, 3);
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].message, "second");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn reader_skips_truncated_or_malformed_jsonl() {
    let directory = test_directory();
    let content = format!(
        "{}not-json\n{{\"unfinished\"",
        event("2026-10-30T10:00:00Z", "INFO", "runtime", "healthy")
    );
    std::fs::write(directory.join("client.0.jsonl"), content).unwrap();
    let page = list_logs(&directory, &LogFilter::default(), 0, 10).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.entries[0].message, "healthy");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn export_respects_filters_and_never_writes_partial_jsonl() {
    let directory = test_directory();
    let content = [
        event("2026-10-30T10:00:00Z", "WARN", "voice", "repeated"),
        event("2026-10-30T10:01:00Z", "INFO", "other", "ignore"),
        event("2026-10-30T10:02:00Z", "WARN", "voice", "repeated"),
        event("2026-10-30T10:03:00Z", "WARN", "voice", "repeated"),
    ]
    .concat();
    std::fs::write(directory.join("client.0.jsonl"), content).unwrap();
    let path = directory.join("export.jsonl");
    let filter = LogFilter {
        level: Some("WARN".to_owned()),
        ..LogFilter::default()
    };
    let result = export_logs(
        &directory,
        &path,
        &filter,
        ExportLimits {
            max_lines: 10,
            max_bytes: 200,
        },
    )
    .unwrap();
    assert!(result.truncated);
    assert!(result.lines > 0 && result.lines < 3);
    assert!(result.bytes <= 200);
    let exported = std::fs::read_to_string(&path).unwrap();
    assert!(
        exported
            .lines()
            .all(|line| serde_json::from_str::<serde_json::Value>(line).is_ok())
    );
    assert!(exported.lines().all(|line| line.contains("WARN")));
    assert_eq!(
        list_logs(&directory, &LogFilter::default(), 0, 20)
            .unwrap()
            .total,
        4
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn export_caps_number_of_complete_lines() {
    let directory = test_directory();
    let content = (0..12)
        .map(|i| {
            event(
                "2026-10-30T10:00:00Z",
                "INFO",
                "runtime",
                &format!("message-{i}"),
            )
        })
        .collect::<String>();
    std::fs::write(directory.join("client.0.jsonl"), content).unwrap();
    let path = directory.join("export.jsonl");
    let result = export_logs(
        &directory,
        &path,
        &LogFilter::default(),
        ExportLimits {
            max_lines: 3,
            max_bytes: 10_000,
        },
    )
    .unwrap();
    assert_eq!(result.lines, 3);
    assert!(result.truncated);
    assert_eq!(std::fs::read_to_string(path).unwrap().lines().count(), 3);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn export_cannot_overwrite_active_archive() {
    let directory = test_directory();
    let path = directory.join("client.0.jsonl");
    let original = event("2026-10-30T10:00:00Z", "INFO", "runtime", "preserved");
    std::fs::write(&path, &original).unwrap();

    let result = export_logs(
        &directory,
        &path,
        &LogFilter::default(),
        ExportLimits::default(),
    );
    assert!(result.is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn export_replaces_hardlink_without_modifying_archive() {
    let directory = test_directory();
    let archive = directory.join("client.0.jsonl");
    let alias = directory.join("export.jsonl");
    let original = event("2026-10-30T10:00:00Z", "INFO", "runtime", "preserved");
    std::fs::write(&archive, &original).unwrap();
    std::fs::hard_link(&archive, &alias).unwrap();

    let filter = LogFilter {
        level: Some("ERROR".to_owned()),
        ..LogFilter::default()
    };
    export_logs(&directory, &alias, &filter, ExportLimits::default()).unwrap();
    assert_eq!(std::fs::read_to_string(&archive).unwrap(), original);
    assert_eq!(std::fs::read_to_string(&alias).unwrap(), "");
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn export_cannot_overwrite_archive_through_symlink() {
    let directory = test_directory();
    let archive = directory.join("client.0.jsonl");
    let alias = directory.join("export.jsonl");
    let original = event("2026-10-30T10:00:00Z", "INFO", "runtime", "preserved");
    std::fs::write(&archive, &original).unwrap();
    std::os::unix::fs::symlink(&archive, &alias).unwrap();

    assert!(
        export_logs(
            &directory,
            &alias,
            &LogFilter::default(),
            ExportLimits::default()
        )
        .is_err()
    );
    assert_eq!(std::fs::read_to_string(&archive).unwrap(), original);
    std::fs::remove_dir_all(directory).unwrap();
}

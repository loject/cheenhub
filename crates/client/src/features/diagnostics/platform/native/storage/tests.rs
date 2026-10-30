use super::*;
use std::path::{Path, PathBuf};

fn test_directory() -> PathBuf {
    let path = std::env::temp_dir().join(format!("cheenhub-diagnostics-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).unwrap().len()
}

#[test]
fn rotation_keeps_only_newest_files_within_size_limit() {
    let directory = test_directory();
    let mut writer = RotatingLog::open(&directory, 3, 64).unwrap();

    for index in 0..40 {
        writer
            .append(format!("{{\"index\":{index}}}\n").as_bytes())
            .unwrap();
    }
    writer.flush().unwrap();

    let files = (0..3)
        .filter_map(|index| {
            let file = directory.join(format!("client.{index}.jsonl"));
            file.exists().then_some(file)
        })
        .collect::<Vec<_>>();
    assert_eq!(files.len(), 3);
    assert!(files.iter().all(|path| file_size(path) <= 64));
    assert!(!directory.join("client.3.jsonl").exists());

    drop(writer);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn reopening_writer_preserves_existing_complete_entries() {
    let directory = test_directory();
    {
        let mut writer = RotatingLog::open(&directory, 3, 256).unwrap();
        writer
            .append(b"{\"message\":\"before restart\"}\n")
            .unwrap();
        writer.flush().unwrap();
    }
    {
        let mut writer = RotatingLog::open(&directory, 3, 256).unwrap();
        writer.append(b"{\"message\":\"after restart\"}\n").unwrap();
        writer.flush().unwrap();
    }

    let content = std::fs::read_to_string(directory.join("client.0.jsonl")).unwrap();
    assert!(content.contains("before restart"));
    assert!(content.contains("after restart"));

    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn oversized_event_is_replaced_by_valid_bounded_json() {
    let directory = test_directory();
    let mut writer = RotatingLog::open(&directory, 3, 128).unwrap();
    let oversized = format!("{{\"message\":\"{}\"}}\n", "x".repeat(256));

    writer.append(oversized.as_bytes()).unwrap();
    writer.flush().unwrap();

    let file = directory.join("client.0.jsonl");
    assert!(file_size(&file) <= 128);
    let content = std::fs::read_to_string(file).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(content.trim_end()).unwrap();
    assert!(
        parsed["fields"]["message"]
            .as_str()
            .unwrap()
            .contains("omitted")
    );

    drop(writer);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn opening_after_partial_crash_line_keeps_following_event_valid() {
    let directory = test_directory();
    std::fs::write(
        directory.join("client.0.jsonl"),
        b"{\"message\":\"complete\"}\n{\"broken\"",
    )
    .unwrap();

    let mut writer = RotatingLog::open(&directory, 3, 256).unwrap();
    writer.append(b"{\"message\":\"new\"}\n").unwrap();
    writer.flush().unwrap();

    let content = std::fs::read_to_string(directory.join("client.0.jsonl")).unwrap();
    let lines = content.lines().collect::<Vec<_>>();
    assert!(lines.iter().any(|line| line.contains("complete")));
    assert!(lines.iter().any(|line| line.contains("new")));
    assert!(
        lines
            .iter()
            .all(|line| serde_json::from_str::<serde_json::Value>(line).is_ok())
    );

    drop(writer);
    std::fs::remove_dir_all(directory).unwrap();
}

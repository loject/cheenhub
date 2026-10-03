use super::files_are_identical;

fn fixture_file(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("cheenhub-updater-{name}-{}", std::process::id()))
}

#[test]
fn detects_identical_files() {
    let left = fixture_file("identical-left");
    let right = fixture_file("identical-right");
    std::fs::write(&left, b"same update binary").expect("left test file should be written");
    std::fs::write(&right, b"same update binary").expect("right test file should be written");

    assert!(files_are_identical(&left, &right).expect("files should be compared"));

    std::fs::remove_file(left).expect("left test file should be removed");
    std::fs::remove_file(right).expect("right test file should be removed");
}

#[test]
fn detects_different_files() {
    let left = fixture_file("different-left");
    let right = fixture_file("different-right");
    std::fs::write(&left, b"old update binary").expect("left test file should be written");
    std::fs::write(&right, b"new update binary").expect("right test file should be written");

    assert!(!files_are_identical(&left, &right).expect("files should be compared"));

    std::fs::remove_file(left).expect("left test file should be removed");
    std::fs::remove_file(right).expect("right test file should be removed");
}

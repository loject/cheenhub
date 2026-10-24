//! Регрессии лимита production-файлов и исключений для тестовых модулей.

use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "cheenhub-file-lines-{}-{}",
            std::process::id(), NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&directory).unwrap();
        Self(directory)
    }

    fn source(&self, relative: &str, lines: usize) -> PathBuf {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "// строка\n".repeat(lines)).unwrap();
        path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn oversized_production_file_is_reported() {
    let directory = TestDirectory::new();
    let path = directory.source("feature.rs", 501);
    let mut oversized = Vec::new();

    check_directory(&directory.0, &mut oversized);

    assert_eq!(oversized, vec![(path, 501)]);
}

#[test]
fn production_file_at_limit_is_accepted() {
    let directory = TestDirectory::new();
    directory.source("feature.rs", 500);
    let mut oversized = Vec::new();

    check_directory(&directory.0, &mut oversized);

    assert!(oversized.is_empty());
}

#[test]
fn test_suites_and_scenario_helpers_are_exempt_from_production_limit() {
    let directory = TestDirectory::new();
    directory.source("feature/tests.rs", 501);
    directory.source("feature/tests/behavior.rs", 700);
    directory.source("feature/tests/support/mod.rs", 600);
    let mut oversized = Vec::new();

    check_directory(&directory.0, &mut oversized);

    assert!(oversized.is_empty(), "test-only files were reported: {oversized:?}");
}

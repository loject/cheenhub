use std::fs;
use std::path::{Path, PathBuf};

const MAX_RUST_FILE_LINES: usize = 500;

pub(crate) fn check_workspace_file_lines() {
    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set"),
    );
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("crate should live under crates/*");
    let crates_dir = workspace_root.join("crates");
    println!("cargo:rerun-if-changed={}", crates_dir.display());

    let mut oversized = Vec::new();
    check_directory(&crates_dir, &mut oversized);

    if oversized.is_empty() {
        return;
    }

    println!(
        "cargo:warning=Rust source files should be {MAX_RUST_FILE_LINES} lines or fewer. Split these files when practical:"
    );
    for (path, line_count) in oversized {
        println!("cargo:warning=  {}: {line_count} lines", path.display());
    }
}

fn check_directory(directory: &Path, oversized: &mut Vec<(PathBuf, usize)>) {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()));

    for entry in entries {
        let path = entry
            .unwrap_or_else(|error| {
                panic!("failed to read entry in {}: {error}", directory.display())
            })
            .path();
        if path.is_dir() {
            check_directory(&path, oversized);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            check_file(&path, oversized);
        }
    }
}

fn check_file(path: &Path, oversized: &mut Vec<(PathBuf, usize)>) {
    let contents = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let line_count = contents.lines().count();
    if line_count > MAX_RUST_FILE_LINES {
        oversized.push((path.to_path_buf(), line_count));
    }
}

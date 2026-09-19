// Проверка Android JNI-вызовов во время сборки.
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const RAW_JNI_CALLS: &[&str] = &[
    ".call_method(",
    ".call_static_method(",
    ".call_nonvirtual_method(",
];

const BEFORE_WINDOW: usize = 1_500;
const AFTER_WINDOW: usize = 3_000;

pub fn check() {
    if env::var_os("CARGO_FEATURE_MOBILE").is_none() {
        return;
    }

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"));
    let source_root = manifest_dir.join("src");

    let mut files = Vec::new();
    collect_android_rust_files(&source_root, &mut files);

    let mut violations = Vec::new();

    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());

        let source = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("failed to read Android Rust source {}: {error}", path.display())
        });

        inspect_source(&path, &source, &mut violations);
    }

    if violations.is_empty() {
        return;
    }

    let mut message = String::from(
        "Обнаружены Android JNI-вызовы без exception guard.\n\
         Каждый прямой JNIEnv::call_* должен проходить через guard_jni_result \
         или guard_media_jni:\n",
    );

    for violation in violations {
        message.push_str("\n  ");
        message.push_str(&violation);
    }

    message.push_str(
        "\n\nPending Java exception нельзя возвращать в Android event loop: \
         это может завершить процесс приложения.",
    );

    panic!("{message}");
}

fn collect_android_rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()));

    for entry in entries {
        let entry = entry.expect("source directory entry is readable");
        let path = entry.path();

        if path.is_dir() {
            collect_android_rust_files(&path, files);
            continue;
        }

        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }

        if is_android_source(&path) {
            files.push(path);
        }
    }
}

fn is_android_source(path: &Path) -> bool {
    path.file_name().and_then(|value| value.to_str()) == Some("android.rs")
        || path
            .components()
            .any(|component| component.as_os_str() == "android")
}

fn inspect_source(path: &Path, source: &str, violations: &mut Vec<String>) {
    for needle in RAW_JNI_CALLS {
        let mut cursor = 0;

        while let Some(relative_offset) = source[cursor..].find(needle) {
            let offset = cursor + relative_offset;

            if !is_guarded(source, offset) {
                let line = source[..offset].bytes().filter(|byte| *byte == b'\n').count() + 1;
                let text = source[offset..]
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .trim();

                violations.push(format!(
                    "{}:{line}: {text}",
                    path.display()
                ));
            }

            cursor = offset + needle.len();
        }
    }
}

fn is_guarded(source: &str, offset: usize) -> bool {
    let before_start = char_boundary_after(
        source,
        offset.saturating_sub(BEFORE_WINDOW),
    );
    let after_end = char_boundary_before(
        source,
        (offset + AFTER_WINDOW).min(source.len()),
    );

    let before = &source[before_start..offset];
    let after = &source[offset..after_end];

    // MediaCodec JNI calls are executed inside this helper, which immediately
    // forwards their Result through guard_jni_result.
    if before.contains("guard_media_jni(") {
        return true;
    }

    // MainActivity/feature bridge calls commonly have to store the JNI Result
    // first to avoid borrowing JNIEnv twice, then guard it immediately after.
    if after.contains("guard_jni_result(") {
        return true;
    }

    // Image picker keeps this small adapter for converting guarded JNI errors
    // to its String error contract.
    after.contains("clear_jni_exception(")
}

fn char_boundary_after(source: &str, mut index: usize) -> usize {
    while index < source.len() && !source.is_char_boundary(index) {
        index += 1;
    }
    index
}

fn char_boundary_before(source: &str, mut index: usize) -> usize {
    while index > 0 && !source.is_char_boundary(index) {
        index -= 1;
    }
    index
}

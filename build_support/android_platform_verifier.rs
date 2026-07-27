// Подготовка JVM helper для проверки TLS через Android Trust Manager.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const PACKAGE_NAME: &str = "rustls-platform-verifier-android";
const ARTIFACT_NAME: &str = "rustls-platform-verifier.aar";
const ANDROID_TARGET: &str = "aarch64-linux-android";

pub(super) fn install() {
    println!("cargo:rerun-if-changed=../../Cargo.lock");
    println!("cargo:rerun-if-changed=../../Cargo.toml");
    println!("cargo:rerun-if-changed=Cargo.toml");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("android") {
        return;
    }

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir is set"));
    let workspace_root = manifest_dir.join("../..");
    let package = package_metadata(&workspace_root);
    let manifest_path = package
        .get("manifest_path")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .expect("rustls-platform-verifier-android manifest path is present");
    let version = package
        .get("version")
        .and_then(serde_json::Value::as_str)
        .expect("rustls-platform-verifier-android version is present");
    let source = manifest_path
        .parent()
        .expect("rustls-platform-verifier-android manifest has a parent")
        .join("maven/rustls/rustls-platform-verifier")
        .join(version)
        .join(format!("rustls-platform-verifier-{version}.aar"));
    let destination = artifact_path(&workspace_root);

    fs::create_dir_all(
        destination
            .parent()
            .expect("Android verifier artifact path has a parent"),
    )
    .expect("Android verifier artifact directory can be created");
    fs::copy(&source, &destination).unwrap_or_else(|error| {
        panic!(
            "failed to copy Android verifier helper from {} to {}: {error}",
            source.display(),
            destination.display()
        )
    });
}

fn package_metadata(workspace_root: &Path) -> serde_json::Value {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--filter-platform",
            ANDROID_TARGET,
            "--manifest-path",
        ])
        .arg(workspace_root.join("Cargo.toml"))
        .output()
        .expect("cargo metadata can locate the Android verifier helper");
    if !output.status.success() {
        panic!(
            "cargo metadata failed while locating Android verifier helper: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata emits valid JSON");
    metadata["packages"]
        .as_array()
        .and_then(|packages| {
            packages
                .iter()
                .find(|package| package["name"].as_str() == Some(PACKAGE_NAME))
        })
        .cloned()
        .expect("rustls-platform-verifier-android is present in Android dependencies")
}

fn artifact_path(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join("target/android-dependencies")
        .join(ARTIFACT_NAME)
}

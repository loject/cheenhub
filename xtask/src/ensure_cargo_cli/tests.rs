use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::{cargo_install_command, install_root, version_output_matches};

#[test]
fn accepts_only_an_exact_version_token() {
    assert!(version_output_matches(
        "dioxus 0.8.0-alpha.1\n",
        "0.8.0-alpha.1"
    ));
    assert!(version_output_matches(
        "dioxus v0.8.0-alpha.1\n",
        "0.8.0-alpha.1"
    ));
    assert!(!version_output_matches(
        "dioxus 0x8x0-alphaX1\n",
        "0.8.0-alpha.1"
    ));
    assert!(!version_output_matches(
        "dioxus 0.8.0-alpha.10\n",
        "0.8.0-alpha.1"
    ));
}

#[test]
fn prefers_configured_install_root_then_cargo_home_then_platform_home() {
    assert_eq!(
        install_root(
            Some(OsStr::new("custom-root")),
            Some(OsStr::new("home")),
            Some(OsStr::new("profile")),
            Some(OsStr::new("profile")),
        )
        .expect("configured root"),
        PathBuf::from("custom-root")
    );
    assert_eq!(
        install_root(
            None,
            Some(OsStr::new("cargo-home")),
            Some(OsStr::new("home")),
            Some(OsStr::new("profile")),
        )
        .expect("CARGO_HOME fallback"),
        PathBuf::from("cargo-home")
    );
    assert_eq!(
        install_root(
            None,
            None,
            Some(OsStr::new("home")),
            Some(OsStr::new("profile"))
        )
        .expect("standard Cargo home fallback"),
        if cfg!(windows) {
            PathBuf::from("profile").join(".cargo")
        } else {
            PathBuf::from("home").join(".cargo")
        }
    );
    assert_eq!(
        install_root(None, None, None, Some(OsStr::new("profile"))).expect("single home fallback"),
        PathBuf::from("profile").join(".cargo")
    );
    assert!(install_root(None, None, None, None).is_err());
}

#[test]
fn builds_locked_forced_install_for_requested_crate_and_version() {
    let command = cargo_install_command(
        Path::new("cargo"),
        Path::new("install-root"),
        "dioxus-cli",
        "0.8.0-alpha.1",
    );
    let arguments = command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    assert_eq!(command.get_program(), "cargo");
    assert_eq!(
        arguments,
        [
            "install",
            "--locked",
            "--force",
            "--root",
            "install-root",
            "--version",
            "0.8.0-alpha.1",
            "dioxus-cli",
        ]
    );
}

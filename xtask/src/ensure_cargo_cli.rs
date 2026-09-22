use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::XtaskResult;

pub(super) fn run(args: Vec<String>) -> XtaskResult<()> {
    let [crate_name, binary_name, expected_version] = args.as_slice() else {
        return Err(
            "Использование: cargo run -p xtask -- ensure-cargo-cli <crate> <binary> <version>."
                .to_owned(),
        );
    };

    let install_root = install_root(
        env::var_os("CARGO_INSTALL_ROOT").as_deref(),
        env::var_os("CARGO_HOME").as_deref(),
        env::var_os("HOME").as_deref(),
        env::var_os("USERPROFILE").as_deref(),
    )?;
    let installed_binary = installed_binary_path(&install_root, binary_name);
    if binary_has_expected_version(&installed_binary, expected_version) {
        return Ok(());
    }

    let cargo = find_in_path("cargo").ok_or_else(|| {
        format!("Cargo не найден: невозможно установить {crate_name} {expected_version}.")
    })?;

    println!(
        "{crate_name} {expected_version} не найден. Устанавливаю его в {}.",
        install_root.display()
    );
    let mut install = cargo_install_command(&cargo, &install_root, crate_name, expected_version);
    let status = install
        .status()
        .map_err(|error| format!("Не удалось запустить cargo install для {crate_name}: {error}"))?;
    if !status.success() {
        return Err(format!(
            "cargo install для {crate_name} завершился со статусом {status}."
        ));
    }

    if !binary_has_expected_version(&installed_binary, expected_version) {
        return Err(format!(
            "{crate_name} {expected_version} установлен некорректно: {} не найден или имеет другую версию.",
            installed_binary.display()
        ));
    }

    println!(
        "{crate_name} {expected_version} установлен и проверен: {}.",
        installed_binary.display()
    );
    Ok(())
}

fn version_output_matches(output: &str, expected_version: &str) -> bool {
    let expected_with_prefix = format!("v{expected_version}");
    output
        .split_whitespace()
        .any(|token| token == expected_version || token == expected_with_prefix)
}

fn binary_has_expected_version(binary: &Path, expected_version: &str) -> bool {
    let Ok(output) = Command::new(binary)
        .arg("--version")
        .stderr(Stdio::null())
        .output()
    else {
        return false;
    };

    output.status.success()
        && version_output_matches(&String::from_utf8_lossy(&output.stdout), expected_version)
}

fn install_root(
    configured_root: Option<&OsStr>,
    cargo_home: Option<&OsStr>,
    home: Option<&OsStr>,
    user_profile: Option<&OsStr>,
) -> XtaskResult<PathBuf> {
    if let Some(configured_root) = configured_root.filter(|root| !root.is_empty()) {
        return Ok(PathBuf::from(configured_root));
    }

    if let Some(cargo_home) = cargo_home.filter(|root| !root.is_empty()) {
        return Ok(PathBuf::from(cargo_home));
    }

    let standard_home = if cfg!(windows) {
        user_profile
            .filter(|root| !root.is_empty())
            .or_else(|| home.filter(|root| !root.is_empty()))
    } else {
        home.filter(|root| !root.is_empty())
            .or_else(|| user_profile.filter(|root| !root.is_empty()))
    };

    standard_home
        .map(|root| PathBuf::from(root).join(".cargo"))
        .ok_or_else(|| {
            "Не заданы CARGO_INSTALL_ROOT, CARGO_HOME, HOME и USERPROFILE: невозможно выбрать каталог установки Cargo."
                .to_owned()
        })
}

fn cargo_install_command(
    cargo: &Path,
    install_root: &Path,
    crate_name: &str,
    expected_version: &str,
) -> Command {
    let mut command = Command::new(cargo);
    command
        .arg("install")
        .arg("--locked")
        .arg("--force")
        .arg("--root")
        .arg(install_root)
        .arg("--version")
        .arg(expected_version)
        .arg(crate_name);
    command
}

fn installed_binary_path(install_root: &Path, binary_name: &str) -> PathBuf {
    install_root
        .join("bin")
        .join(format!("{binary_name}{}", env::consts::EXE_SUFFIX))
}

fn find_in_path(binary_name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let executable_name = format!("{binary_name}{}", env::consts::EXE_SUFFIX);

    env::split_paths(&path)
        .map(|directory| directory.join(&executable_name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
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
            install_root(None, None, None, Some(OsStr::new("profile")))
                .expect("single home fallback"),
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
}

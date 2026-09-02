//! Прямая установка DEB-обновления без отдельного update-helper.

use dioxus::prelude::info;
use std::path::Path;
use std::process::Command;

use crate::features::application_update::DownloadedUpdate;

#[derive(Debug)]
struct DebPackageMetadata {
    package: String,
    version: String,
    architecture: String,
}

pub(super) fn is_deb_update(file: &DownloadedUpdate) -> bool {
    Path::new(&file.path)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("deb"))
}

pub(super) fn install_deb_update(
    expected_version: &str,
    file: &DownloadedUpdate,
) -> Result<(), String> {
    use std::os::unix::process::CommandExt;

    let installer_path = Path::new(&file.path);
    if !installer_path.is_file() {
        return Err(format!(
            "Файл DEB-обновления не найден: {}.",
            installer_path.display()
        ));
    }

    let current_exe = std::env::current_exe()
        .map_err(|error| format!("Не удалось определить путь к текущему CheenHub: {error}"))?;

    if !current_exe.is_file() {
        return Err(format!(
            "Не удалось найти исполняемый файл CheenHub: {}.",
            current_exe.display()
        ));
    }

    let metadata = validate_deb_package(installer_path, expected_version)?;

    info!(
        package = %metadata.package,
        package_version = %metadata.version,
        architecture = %metadata.architecture,
        path = %installer_path.display(),
        "validated DEB update package"
    );

    if !command_exists("pkexec") {
        return Err(
            "Автоматическая установка DEB-обновления недоступна: в системе не найден pkexec."
                .to_owned(),
        );
    }

    if !command_exists("apt") {
        return Err(
            "Автоматическая установка DEB-обновления недоступна: в системе не найден apt."
                .to_owned(),
        );
    }

    info!(
        package = %metadata.package,
        package_version = %metadata.version,
        path = %installer_path.display(),
        "running apt install for DEB update"
    );

    let status = Command::new("pkexec")
        .arg("apt")
        .arg("install")
        .arg("-y")
        .arg(installer_path)
        .status()
        .map_err(|error| format!("Не удалось запустить установку DEB-обновления: {error}"))?;

    if !status.success() {
        return Err(format!(
            "Установка DEB-обновления завершилась с кодом {}.",
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_owned())
        ));
    }

    verify_installed_package(&metadata)?;

    if !current_exe.is_file() {
        return Err(format!(
            "DEB-пакет установлен, но новый исполняемый файл CheenHub не найден: {}.",
            current_exe.display()
        ));
    }

    info!(
        package = %metadata.package,
        package_version = %metadata.version,
        executable = %current_exe.display(),
        "DEB update installed; replacing current process with updated CheenHub"
    );

    // После успешного apt install путь current_exe уже указывает на новый
    // бинарник из установленного DEB. exec() заменяет текущий образ процесса
    // новым, сохраняя PID. При успехе этот вызов никогда не возвращается.
    let error = Command::new(&current_exe).exec();

    Err(format!(
        "DEB-обновление установлено, но не удалось перезапустить CheenHub через exec: {error}"
    ))
}

fn validate_deb_package(
    installer_path: &Path,
    expected_version: &str,
) -> Result<DebPackageMetadata, String> {
    if !command_exists("dpkg-deb") {
        return Err("Не удалось проверить DEB-пакет: в системе не найден dpkg-deb.".to_owned());
    }

    let metadata = DebPackageMetadata {
        package: read_deb_field(installer_path, "Package")?,
        version: read_deb_field(installer_path, "Version")?,
        architecture: read_deb_field(installer_path, "Architecture")?,
    };

    let normalized_package: String = metadata
        .package
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|character| character.to_ascii_lowercase())
        .collect();

    if !matches!(normalized_package.as_str(), "cheenhub" | "cheenhubclient") {
        return Err(format!(
            "Скачанный DEB содержит неожиданный пакет `{}`. Установка отменена.",
            metadata.package
        ));
    }

    if !deb_version_matches(&metadata.version, expected_version) {
        return Err(format!(
            "Версия DEB не совпадает с GitHub Release: ожидалась {expected_version}, получена {}.",
            metadata.version
        ));
    }

    #[cfg(target_arch = "x86_64")]
    if metadata.architecture != "amd64" {
        return Err(format!(
            "DEB имеет неподходящую архитектуру: ожидалась amd64, получена {}.",
            metadata.architecture
        ));
    }

    Ok(metadata)
}

fn verify_installed_package(metadata: &DebPackageMetadata) -> Result<(), String> {
    if !command_exists("dpkg-query") {
        return Err(
            "DEB установлен, но проверить установленную версию невозможно: в системе не найден dpkg-query."
                .to_owned(),
        );
    }

    let output = Command::new("dpkg-query")
        .arg("-W")
        .arg("-f=${Version}")
        .arg(&metadata.package)
        .output()
        .map_err(|error| {
            format!(
                "DEB установлен, но не удалось проверить пакет `{}` через dpkg-query: {error}",
                metadata.package
            )
        })?;

    if !output.status.success() {
        return Err(format!(
            "DEB-установщик завершился успешно, но dpkg не видит пакет `{}`: {}",
            metadata.package,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let installed_version = String::from_utf8_lossy(&output.stdout).trim().to_owned();

    if installed_version != metadata.version {
        return Err(format!(
            "После установки версия пакета `{}` не совпадает с DEB: ожидалась {}, установлена {}.",
            metadata.package, metadata.version, installed_version
        ));
    }

    info!(
        package = %metadata.package,
        version = %installed_version,
        "verified installed DEB package version"
    );

    Ok(())
}

fn read_deb_field(installer_path: &Path, field: &str) -> Result<String, String> {
    let output = Command::new("dpkg-deb")
        .arg("--field")
        .arg(installer_path)
        .arg(field)
        .output()
        .map_err(|error| format!("Не удалось прочитать поле {field} из DEB: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "Не удалось прочитать поле {field} из DEB: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();

    if value.is_empty() {
        return Err(format!("В DEB отсутствует поле {field}."));
    }

    Ok(value)
}

fn deb_version_matches(actual: &str, expected: &str) -> bool {
    let actual = actual
        .split_once(':')
        .map(|(_, version)| version)
        .unwrap_or(actual);

    actual == expected
        || actual
            .strip_prefix(expected)
            .is_some_and(|suffix| suffix.starts_with('-') || suffix.starts_with('+'))
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name} >/dev/null 2>&1"))
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    use super::deb_version_matches;

    #[test]
    fn accepts_debian_revision_for_release_version() {
        assert!(deb_version_matches("0.24.1-1", "0.24.1"));
        assert!(deb_version_matches("1:0.24.1-1", "0.24.1"));
        assert!(deb_version_matches("0.24.1+build1", "0.24.1"));
    }

    #[test]
    fn rejects_different_release_version() {
        assert!(!deb_version_matches("0.24.0-1", "0.24.1"));
        assert!(!deb_version_matches("0.25.0", "0.24.1"));
    }
}

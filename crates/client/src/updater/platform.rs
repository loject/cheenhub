//! Платформенные операции отдельного апдейтера.

use std::io::{BufReader, Read};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

#[cfg(not(target_os = "linux"))]
pub(super) fn wait_for_process_exit(pid: u32) {
    for _ in 0..120 {
        if !is_process_running(pid) {
            return;
        }
        thread::sleep(Duration::from_millis(500));
    }
}

#[cfg(target_os = "linux")]
pub(super) fn stop_process_and_wait(pid: u32) -> Result<(), String> {
    if !is_process_running(pid) {
        return Ok(());
    }

    let status = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .map_err(|error| format!("Не удалось завершить текущий CheenHub: {error}"))?;

    if !status.success() && is_process_running(pid) {
        return Err(format!(
            "Не удалось отправить сигнал завершения CheenHub (PID {pid})."
        ));
    }

    for _ in 0..120 {
        if !is_process_running(pid) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }

    Err(
        "Обновление установлено, но текущий CheenHub не завершился вовремя. Перезапуск отменён, чтобы не запускать две копии приложения."
            .to_owned(),
    )
}

pub(super) fn run_installer(
    installer_path: &Path,
    expected_version: Option<&str>,
    mut on_log: impl FnMut(&str),
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    if installer_path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("deb"))
    {
        let metadata = validate_deb_package(installer_path, expected_version)?;
        on_log(&format!(
            "validated deb package: package={}, version={}, architecture={}",
            metadata.package, metadata.version, metadata.architecture
        ));
    }

    #[cfg(not(target_os = "linux"))]
    let _ = expected_version;

    let mut command = installer_command(installer_path)?;
    on_log(&format!("running update installer command: {command:?}"));
    let status = command
        .status()
        .map_err(|error| format!("Не удалось запустить установщик обновления: {error}"))?;

    if status.success() {
        return Ok(());
    }

    Err(format!(
        "Установщик обновления завершился с кодом {}.",
        status
            .code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| "unknown".to_owned())
    ))
}

pub(super) fn restart_application(restart_path: &Path) -> Result<(), String> {
    Command::new(restart_path)
        .spawn()
        .map_err(|error| format!("Не удалось перезапустить CheenHub: {error}"))?;
    Ok(())
}

pub(super) fn verify_installed_application(restart_path: &Path) -> Result<(), String> {
    if !restart_path.is_file() {
        return Err(format!(
            "Установщик завершился, но исполняемый файл CheenHub не найден: {}.",
            restart_path.display()
        ));
    }

    let updater_path = std::env::current_exe().map_err(|error| {
        format!("Не удалось проверить исполняемый файл CheenHub после установки: {error}")
    })?;
    if updater_path == restart_path {
        return Ok(());
    }

    if files_are_identical(&updater_path, restart_path)? {
        return Err(
            "Установщик завершился, но исполняемый файл CheenHub не был обновлен.".to_owned(),
        );
    }

    Ok(())
}

fn files_are_identical(left: &Path, right: &Path) -> Result<bool, String> {
    let left_file = std::fs::File::open(left).map_err(|error| {
        format!(
            "Не удалось открыть updater {} для проверки обновления: {error}",
            left.display()
        )
    })?;
    let right_file = std::fs::File::open(right).map_err(|error| {
        format!(
            "Не удалось открыть установленный файл {} для проверки обновления: {error}",
            right.display()
        )
    })?;
    let left_len = left_file
        .metadata()
        .map_err(|error| format!("Не удалось прочитать размер updater-а: {error}"))?
        .len();
    let right_len = right_file
        .metadata()
        .map_err(|error| format!("Не удалось прочитать размер установленного файла: {error}"))?
        .len();
    if left_len != right_len {
        return Ok(false);
    }

    let mut left_reader = BufReader::new(left_file);
    let mut right_reader = BufReader::new(right_file);
    let mut left_buffer = [0_u8; 64 * 1024];
    let mut right_buffer = [0_u8; 64 * 1024];
    loop {
        let left_read = left_reader
            .read(&mut left_buffer)
            .map_err(|error| format!("Не удалось прочитать updater для проверки: {error}"))?;
        let right_read = right_reader.read(&mut right_buffer).map_err(|error| {
            format!("Не удалось прочитать установленный файл для проверки: {error}")
        })?;
        if left_read != right_read || left_buffer[..left_read] != right_buffer[..right_read] {
            return Ok(false);
        }
        if left_read == 0 {
            return Ok(true);
        }
    }
}

#[cfg(target_os = "windows")]
fn installer_command(installer_path: &Path) -> Result<Command, String> {
    let extension = installer_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if extension == "msi" {
        let mut command = Command::new("msiexec");
        command
            .arg("/i")
            .arg(installer_path)
            .arg("/passive")
            .arg("/norestart");
        return Ok(command);
    }

    let mut command = Command::new(installer_path);
    command.arg("/S").arg("/SKIP_WEBVIEW2");
    Ok(command)
}

#[cfg(target_os = "linux")]
fn installer_command(installer_path: &Path) -> Result<Command, String> {
    let file_name = installer_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();

    if file_name.ends_with(".AppImage") {
        make_executable(installer_path)?;
        return Ok(Command::new(installer_path));
    }

    if file_name.ends_with(".deb") {
        if !command_exists("pkexec") {
            return Err(
                "Автоматическая установка обновления недоступна: в системе не найден pkexec."
                    .to_owned(),
            );
        }

        let mut command = Command::new("pkexec");
        command
            .arg("apt")
            .arg("install")
            .arg("-y")
            .arg(installer_path);
        return Ok(command);
    }

    let mut command = Command::new("xdg-open");
    command.arg(installer_path);
    Ok(command)
}

#[cfg(target_os = "macos")]
fn installer_command(installer_path: &Path) -> Result<Command, String> {
    let mut command = Command::new("open");
    command.arg(installer_path);
    Ok(command)
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn installer_command(_installer_path: &Path) -> Result<Command, String> {
    Err("Установка обновления недоступна на этой платформе.".to_owned())
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct DebPackageMetadata {
    package: String,
    version: String,
    architecture: String,
}

#[cfg(target_os = "linux")]
fn validate_deb_package(
    installer_path: &Path,
    expected_version: Option<&str>,
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

    if let Some(expected_version) = expected_version
        && !deb_version_matches(&metadata.version, expected_version)
    {
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "windows")]
fn is_process_running(pid: u32) -> bool {
    let Ok(output) = Command::new("tasklist")
        .arg("/FI")
        .arg(format!("PID eq {pid}"))
        .arg("/FO")
        .arg("CSV")
        .arg("/NH")
        .output()
    else {
        return false;
    };

    String::from_utf8_lossy(&output.stdout).contains(&format!("\"{pid}\""))
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn is_process_running(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn is_process_running(_pid: u32) -> bool {
    false
}

#[cfg(target_os = "linux")]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = std::fs::metadata(path)
        .map_err(|error| format!("Не удалось проверить права файла обновления: {error}"))?
        .permissions();
    permissions.set_mode(permissions.mode() | 0o755);
    std::fs::set_permissions(path, permissions)
        .map_err(|error| format!("Не удалось подготовить AppImage к запуску: {error}"))
}

#[cfg(target_os = "linux")]
fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name} >/dev/null 2>&1"))
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    use super::files_are_identical;

    fn test_file(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("cheenhub-updater-{name}-{}", std::process::id()))
    }

    #[test]
    fn detects_identical_files() {
        let left = test_file("identical-left");
        let right = test_file("identical-right");
        std::fs::write(&left, b"same update binary").expect("left test file should be written");
        std::fs::write(&right, b"same update binary").expect("right test file should be written");

        assert!(files_are_identical(&left, &right).expect("files should be compared"));

        std::fs::remove_file(left).expect("left test file should be removed");
        std::fs::remove_file(right).expect("right test file should be removed");
    }

    #[test]
    fn detects_different_files() {
        let left = test_file("different-left");
        let right = test_file("different-right");
        std::fs::write(&left, b"old update binary").expect("left test file should be written");
        std::fs::write(&right, b"new update binary").expect("right test file should be written");

        assert!(!files_are_identical(&left, &right).expect("files should be compared"));

        std::fs::remove_file(left).expect("left test file should be removed");
        std::fs::remove_file(right).expect("right test file should be removed");
    }
}

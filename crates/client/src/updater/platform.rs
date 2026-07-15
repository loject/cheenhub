//! Платформенные операции отдельного апдейтера.

use std::io::{BufReader, Read};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub(super) fn wait_for_process_exit(pid: u32) {
    for _ in 0..120 {
        if !is_process_running(pid) {
            return;
        }
        thread::sleep(Duration::from_millis(500));
    }
}

pub(super) fn run_installer(
    installer_path: &Path,
    mut on_log: impl FnMut(&str),
) -> Result<(), String> {
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
        let mut command = if command_exists("pkexec") {
            let mut command = Command::new("pkexec");
            command.arg("dpkg").arg("-i");
            command
        } else {
            Command::new("xdg-open")
        };
        command.arg(installer_path);
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

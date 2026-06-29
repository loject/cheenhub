//! Платформенные операции отдельного апдейтера.

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

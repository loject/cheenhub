//! Выбор платформенной реализации скачивания обновлений.

#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
use std::io::Write;
#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
use std::path::{Path, PathBuf};
#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
use std::process::Command;

#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
use dioxus::prelude::*;

#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
use sha2::{Digest, Sha256};

#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
use super::USER_AGENT;
#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
use super::{PrimaryUpdateActionPresentation, unavailable_action_presentation};
#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
use crate::features::application_update::types::UpdateDownloadOutcome;
#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
use crate::features::application_update::{
    AvailableUpdate, DownloadedUpdate, UpdateDownloadAsset, UpdateDownloadProgress,
    UpdateDownloadStatus,
};

#[cfg(all(not(target_family = "wasm"), target_os = "linux"))]
mod linux_deb;

#[cfg(all(not(target_family = "wasm"), target_os = "android"))]
#[path = "android.rs"]
mod android;

#[cfg(all(not(target_family = "wasm"), target_os = "android"))]
pub(crate) use android::{
    download_update_asset, install_downloaded_update, opens_download_externally,
    primary_action_presentation, select_update_asset,
};

#[cfg(target_family = "wasm")]
pub(crate) use super::web::{
    download_update_asset, install_downloaded_update, opens_download_externally,
    primary_action_presentation, select_update_asset,
};

#[cfg(all(
    not(target_family = "wasm"),
    target_os = "windows",
    target_arch = "x86_64"
))]
const PREFERRED_SUFFIXES: &[&str] = &["windows-x64-setup.exe", "windows-x64.msi"];
#[cfg(all(
    not(target_family = "wasm"),
    target_os = "linux",
    target_arch = "x86_64"
))]
const PREFERRED_SUFFIXES: &[&str] = &["linux-x64.deb", "linux-x64.AppImage"];
#[cfg(all(
    not(target_family = "wasm"),
    not(all(target_os = "windows", target_arch = "x86_64")),
    not(all(target_os = "linux", target_arch = "x86_64")),
    not(target_os = "android")
))]
const PREFERRED_SUFFIXES: &[&str] = &[];

#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
pub(crate) fn select_update_asset(assets: &[UpdateDownloadAsset]) -> Option<UpdateDownloadAsset> {
    for suffix in PREFERRED_SUFFIXES {
        if let Some(asset) = assets
            .iter()
            .find(|asset| asset.name.ends_with(suffix))
            .cloned()
        {
            return Some(asset);
        }
    }

    None
}

#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
/// Сообщает, открывает ли платформа загрузку во внешнем приложении.
pub(crate) const fn opens_download_externally() -> bool {
    false
}

#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
/// Возвращает desktop-представление основного действия обновления.
pub(crate) fn primary_action_presentation(
    update: &AvailableUpdate,
    download_status: &UpdateDownloadStatus,
) -> PrimaryUpdateActionPresentation {
    match download_status {
        UpdateDownloadStatus::Downloading { version, .. } if version == &update.version => {
            PrimaryUpdateActionPresentation {
                label: "Скачиваем...",
                disabled: true,
                installs_downloaded: false,
                requested_message: "Начинаем скачивание обновления.",
            }
        }
        UpdateDownloadStatus::Downloaded { version, .. } if version == &update.version => {
            PrimaryUpdateActionPresentation {
                label: "Установить",
                disabled: false,
                installs_downloaded: true,
                requested_message: "Запускаем установщик обновления.",
            }
        }
        _ if update.download_asset.is_none() => unavailable_action_presentation(),
        _ => PrimaryUpdateActionPresentation {
            label: "Скачать обновление",
            disabled: false,
            installs_downloaded: false,
            requested_message: "Начинаем скачивание обновления.",
        },
    }
}

#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
pub(crate) async fn download_update_asset(
    asset: UpdateDownloadAsset,
    mut on_progress: impl FnMut(UpdateDownloadProgress) + 'static,
) -> Result<UpdateDownloadOutcome, String> {
    let file_name = safe_file_name(&asset.name)?;
    let download_dir = default_download_dir()?;
    std::fs::create_dir_all(&download_dir).map_err(|error| {
        format!(
            "Не удалось подготовить папку загрузок {}: {error}",
            download_dir.display()
        )
    })?;

    #[cfg(target_os = "linux")]
    cleanup_update_cache(&download_dir);

    let destination = unique_destination(&download_dir, &file_name);
    info!(
        asset_name = %asset.name,
        asset_size_bytes = asset.size_bytes,
        destination = %destination.display(),
        "downloading application update asset"
    );

    let mut response = reqwest::Client::new()
        .get(&asset.download_url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .header(reqwest::header::ACCEPT, "application/octet-stream")
        .send()
        .await
        .map_err(|error| format!("Не удалось начать скачивание обновления: {error}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "GitHub вернул ответ {} при скачивании обновления.",
            response.status()
        ));
    }

    let total_bytes = response
        .content_length()
        .or_else(|| (asset.size_bytes > 0).then_some(asset.size_bytes));
    let mut downloaded_bytes = 0_u64;
    let mut hasher = Sha256::new();
    let mut file = std::fs::File::create(&destination).map_err(|error| {
        format!(
            "Не удалось сохранить обновление в {}: {error}",
            destination.display()
        )
    })?;

    on_progress(UpdateDownloadProgress {
        downloaded_bytes,
        total_bytes,
        bytes_per_second: 0,
    });

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("Не удалось прочитать файл обновления: {error}"))?
    {
        file.write_all(&chunk).map_err(|error| {
            format!(
                "Не удалось записать обновление в {}: {error}",
                destination.display()
            )
        })?;
        hasher.update(&chunk);
        downloaded_bytes = downloaded_bytes.saturating_add(chunk.len() as u64);
        on_progress(UpdateDownloadProgress {
            downloaded_bytes,
            total_bytes,
            bytes_per_second: 0,
        });
    }

    file.flush().map_err(|error| {
        format!(
            "Не удалось сохранить обновление в {}: {error}",
            destination.display()
        )
    })?;

    let actual_digest = format!("{:x}", hasher.finalize());
    if let Some(expected_digest) = asset.digest.as_deref() {
        let expected_digest = expected_digest.strip_prefix("sha256:").ok_or_else(|| {
            format!("GitHub вернул неподдерживаемый формат контрольной суммы: {expected_digest}.")
        })?;

        if !actual_digest.eq_ignore_ascii_case(expected_digest) {
            drop(file);
            let _ = std::fs::remove_file(&destination);
            return Err(
                "SHA-256 скачанного обновления не совпадает с GitHub Release. Файл удалён, установка отменена."
                    .to_owned(),
            );
        }

        info!(
            path = %destination.display(),
            "application update SHA-256 verified"
        );
    } else {
        warn!(
            asset_name = %asset.name,
            "GitHub release asset does not contain SHA-256 digest"
        );
    }

    info!(
        asset_name = %asset.name,
        saved_path = %destination.display(),
        saved_bytes = downloaded_bytes,
        "downloaded application update asset"
    );

    Ok(UpdateDownloadOutcome {
        downloaded_file: Some(DownloadedUpdate {
            file_name,
            path: destination.display().to_string(),
        }),
    })
}

#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
pub(crate) fn install_downloaded_update(
    version: &str,
    file: &DownloadedUpdate,
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    if linux_deb::is_deb_update(file) {
        info!(
            update_version = %version,
            update_path = %file.path,
            "installing DEB update directly from the running application"
        );

        return linux_deb::install_deb_update(version, file);
    }

    info!(
        update_version = %version,
        update_path = %file.path,
        "starting application update helper"
    );
    start_updater(version, file)
}

#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
fn start_updater(version: &str, file: &DownloadedUpdate) -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|error| {
        format!("Не удалось определить путь к текущему приложению перед обновлением: {error}")
    })?;
    if !current_exe.is_file() {
        return Err(format!(
            "Не удалось найти исполняемый файл CheenHub для запуска обновления: {}.",
            current_exe.display()
        ));
    }

    let updater_exe = prepare_updater_executable(&current_exe)?;

    let spawn_result = Command::new(&updater_exe)
        .arg("--cheenhub-update")
        .arg("--installer")
        .arg(&file.path)
        .arg("--app-pid")
        .arg(std::process::id().to_string())
        .arg("--restart")
        .arg(&current_exe)
        .arg("--version")
        .arg(version)
        .spawn();

    if let Err(error) = spawn_result {
        cleanup_failed_updater_copy(&updater_exe, &current_exe);
        return Err(format!(
            "Не удалось запустить режим обновления CheenHub: {error}"
        ));
    }

    info!(
        update_version = %version,
        updater_path = %updater_exe.display(),
        restart_path = %current_exe.display(),
        "application update mode started; main window should close"
    );
    Ok(())
}

#[cfg(all(not(target_family = "wasm"), target_os = "windows"))]
fn prepare_updater_executable(current_exe: &Path) -> Result<PathBuf, String> {
    let helpers_dir = std::env::temp_dir().join("cheenhub-update-helpers");
    if helpers_dir.exists()
        && let Err(error) = std::fs::remove_dir_all(&helpers_dir)
    {
        warn!(
            path = %helpers_dir.display(),
            %error,
            "failed to remove stale application update helpers"
        );
    }
    std::fs::create_dir_all(&helpers_dir).map_err(|error| {
        format!(
            "Не удалось подготовить временную папку updater-а {}: {error}",
            helpers_dir.display()
        )
    })?;

    let updater_exe =
        helpers_dir.join(format!("cheenhub-update-helper-{}.exe", std::process::id()));
    std::fs::copy(current_exe, &updater_exe).map_err(|error| {
        format!(
            "Не удалось подготовить временную копию updater-а {}: {error}",
            updater_exe.display()
        )
    })?;
    info!(
        source = %current_exe.display(),
        destination = %updater_exe.display(),
        "prepared detached application update helper"
    );
    Ok(updater_exe)
}

#[cfg(all(
    not(target_family = "wasm"),
    not(target_os = "windows"),
    not(target_os = "android")
))]
fn prepare_updater_executable(current_exe: &Path) -> Result<PathBuf, String> {
    Ok(current_exe.to_path_buf())
}

#[cfg(all(not(target_family = "wasm"), target_os = "windows"))]
fn cleanup_failed_updater_copy(updater_exe: &Path, current_exe: &Path) {
    if updater_exe == current_exe {
        return;
    }
    if let Err(error) = std::fs::remove_file(updater_exe) {
        warn!(
            path = %updater_exe.display(),
            %error,
            "failed to remove application update helper after launch failure"
        );
    }
}

#[cfg(all(
    not(target_family = "wasm"),
    not(target_os = "windows"),
    not(target_os = "android")
))]
fn cleanup_failed_updater_copy(_updater_exe: &Path, _current_exe: &Path) {}

#[cfg(all(
    not(target_family = "wasm"),
    not(target_os = "android"),
    target_os = "linux"
))]
fn default_download_dir() -> Result<PathBuf, String> {
    if let Some(cache_home) = std::env::var_os("XDG_CACHE_HOME") {
        let cache_home = PathBuf::from(cache_home);
        if cache_home.is_absolute() {
            return Ok(cache_home.join("cheenhub").join("updates"));
        }
    }

    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home)
            .join(".cache")
            .join("cheenhub")
            .join("updates"));
    }

    Ok(std::env::temp_dir().join("cheenhub").join("updates"))
}

#[cfg(all(
    not(target_family = "wasm"),
    not(target_os = "android"),
    not(target_os = "linux")
))]
fn default_download_dir() -> Result<PathBuf, String> {
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        return Ok(PathBuf::from(profile).join("Downloads"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home).join("Downloads"));
    }

    std::env::current_dir()
        .map_err(|error| format!("Не удалось определить папку для загрузки обновления: {error}"))
}

#[cfg(target_os = "linux")]
fn cleanup_update_cache(directory: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        if let Err(error) = std::fs::remove_file(&path) {
            warn!(
                path = %path.display(),
                %error,
                "failed to remove stale application update"
            );
        }
    }
}

#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
fn safe_file_name(name: &str) -> Result<String, String> {
    let Some(file_name) = Path::new(name).file_name().and_then(|value| value.to_str()) else {
        return Err("GitHub вернул некорректное имя файла обновления.".to_owned());
    };
    if file_name.trim().is_empty() {
        return Err("GitHub вернул пустое имя файла обновления.".to_owned());
    }

    Ok(file_name.to_owned())
}

#[cfg(all(not(target_family = "wasm"), not(target_os = "android")))]
fn unique_destination(directory: &Path, file_name: &str) -> PathBuf {
    let destination = directory.join(file_name);
    if !destination.exists() {
        return destination;
    }

    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(file_name);
    let extension = path.extension().and_then(|value| value.to_str());
    for index in 1..100 {
        let candidate_name = match extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = directory.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    directory.join(file_name)
}

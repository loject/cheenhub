//! Инициализация постоянного tracing и файлов Dioxus-профилирования.

use std::fs::File;
use std::path::{Path, PathBuf};

use dioxus::logger::tracing::{dispatcher, subscriber::set_global_default};
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
};

use super::logger::{diagnostic_log_filter, start_archive_worker};

const ARCHIVE_FILES: usize = 5;
const ARCHIVE_FILE_BYTES: u64 = 5 * 1024 * 1024;
const PROFILE_ENV: &str = "CHEENHUB_DIOXUS_PROFILE";
const PROFILE_FILTER_ENV: &str = "CHEENHUB_DIOXUS_PROFILE_FILTER";
const PROFILE_LOG_ENV: &str = "CHEENHUB_DIOXUS_PROFILE_LOG";
const PROFILE_DEFAULT_FILTER: &str = concat!(
    "info,",
    "dioxus_core::scope_arena=trace,",
    "dioxus_core::virtual_dom=trace,",
    "dioxus_core::runtime=trace,",
    "cheenhub::features::voice_chat=trace,",
    "cheenhub::features::social=trace",
);

/// Подключает постоянные логи до запуска Dioxus и первого сетевого соединения.
pub(super) fn initialize() {
    if dispatcher::has_been_set() {
        return;
    }

    let archive_layer = archive_directory().and_then(|directory| {
        match start_archive_worker(&directory, ARCHIVE_FILES, ARCHIVE_FILE_BYTES) {
            Ok((writer, worker)) => {
                // Подписчик удерживает отправитель; фоновый поток живёт до завершения процесса.
                drop(worker);
                let filter_override = std::env::var("CHEENHUB_CLIENT_LOG_FILTER").ok();
                Some(
                    fmt::layer()
                        .json()
                        .with_writer(writer)
                        .with_filter(diagnostic_log_filter(filter_override.as_deref())),
                )
            }
            Err(error) => {
                eprintln!(
                    "Не удалось открыть каталог логов {}: {error}",
                    directory.display()
                );
                None
            }
        }
    });

    // Старый профиль Dioxus остаётся отдельным текстовым файлом со span CLOSE.
    let profile_layer = profile_file().map(|(file, filter)| {
        fmt::layer()
            .with_ansi(false)
            .with_span_events(FmtSpan::CLOSE)
            .with_target(true)
            .with_writer(move || file.try_clone().expect("profile file handle remains valid"))
            .with_filter(filter)
    });
    let console_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let console_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .with_filter(console_filter);

    let subscriber = tracing_subscriber::registry()
        .with(archive_layer)
        .with(profile_layer)
        .with(console_layer);
    if let Err(error) = set_global_default(subscriber) {
        eprintln!("Не удалось инициализировать диагностику tracing: {error}");
    }
}

fn archive_directory() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
        windows_log_directory(local_app_data.as_deref())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let state_home = std::env::var_os("XDG_STATE_HOME").map(PathBuf::from);
        let home = std::env::var_os("HOME").map(PathBuf::from);
        unix_log_directory(state_home.as_deref(), home.as_deref())
    }
}

fn profile_file() -> Option<(File, EnvFilter)> {
    std::env::var_os(PROFILE_ENV)?;
    let path = std::env::var_os(PROFILE_LOG_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/dioxus-profile.log")
        });
    let result = (|| {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        File::create(&path)
    })();
    let file = match result {
        Ok(file) => file,
        Err(error) => {
            eprintln!(
                "Не удалось открыть Dioxus profile {}: {error}",
                path.display()
            );
            return None;
        }
    };
    let filter_text =
        std::env::var(PROFILE_FILTER_ENV).unwrap_or_else(|_| PROFILE_DEFAULT_FILTER.to_owned());
    let filter = EnvFilter::try_new(&filter_text).unwrap_or_else(|error| {
        eprintln!("Некорректный Dioxus profile filter '{filter_text}': {error}");
        EnvFilter::new(PROFILE_DEFAULT_FILTER)
    });
    eprintln!("Dioxus profile log: {}", path.display());
    Some((file, filter))
}

/// Выбирает стандартный каталог состояния Linux с запасным вариантом в HOME.
#[cfg(not(target_os = "windows"))]
fn unix_log_directory(state_home: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
    state_home
        .filter(|path| path.is_absolute())
        .map(|path| path.join("cheenhub/logs"))
        .or_else(|| home.map(|path| path.join(".local/state/cheenhub/logs")))
}

/// Выбирает каталог логов внутри LocalAppData Windows.
#[cfg(target_os = "windows")]
fn windows_log_directory(local_app_data: Option<&Path>) -> Option<PathBuf> {
    local_app_data.map(|path| path.join("CheenHub").join("logs"))
}

#[cfg(test)]
#[path = "bootstrap/tests.rs"]
mod tests;

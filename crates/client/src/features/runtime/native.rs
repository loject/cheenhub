//! Native-реализация runtime-помощников.

use std::time::Duration;

/// Асинхронно ожидает указанную продолжительность через Tokio runtime.
#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "desktop", feature = "mobile")
))]
pub(super) async fn sleep_duration(duration: Duration) {
    tokio::time::sleep(duration).await;
}

/// Асинхронно ожидает указанную продолжительность через browser timer.
#[cfg(target_arch = "wasm32")]
pub(super) async fn sleep_duration(duration: Duration) {
    super::web::sleep_duration(duration).await;
}

/// Ожидает указанную продолжительность в host-проверках без native runtime.
#[cfg(all(
    not(target_arch = "wasm32"),
    not(any(feature = "desktop", feature = "mobile"))
))]
pub(super) async fn sleep_duration(duration: Duration) {
    super::unsupported::sleep_duration(duration).await;
}

/// Запускает desktop-клиент с размером окна CheenHub.
#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
pub(super) fn launch_client(app: fn() -> dioxus::prelude::Element) {
    use dioxus::desktop::{
        Config, LogicalSize, WindowBuilder, WindowCloseBehaviour, icon_from_memory,
    };

    init_dioxus_profile_logger();

    const WINDOW_WIDTH: f64 = 1280.0;
    const WINDOW_HEIGHT: f64 = 820.0;
    const WINDOW_MIN_WIDTH: f64 = 960.0;
    const WINDOW_MIN_HEIGHT: f64 = 640.0;

    let icon = icon_from_memory(include_bytes!(
        "../../../../../crates/client/public/icons/icon-512.png"
    ))
    .expect("failed to load window icon");

    let window = WindowBuilder::new()
        .with_title("CheenHub")
        .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .with_min_inner_size(LogicalSize::new(WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT))
        .with_maximized(true);
    let close_behaviour = if crate::features::system_tray::load_minimize_to_tray_on_close() {
        WindowCloseBehaviour::WindowHides
    } else {
        WindowCloseBehaviour::WindowCloses
    };

    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(window)
                .with_icon(icon)
                .with_close_behaviour(close_behaviour)
                .with_menu(None),
        )
        .launch(app);
}

#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
fn init_dioxus_profile_logger() {
    use dioxus::logger::tracing::{dispatcher, info, subscriber::set_global_default};
    use std::path::PathBuf;
    use tracing_subscriber::{EnvFilter, FmtSubscriber, fmt::format::FmtSpan};

    const ENABLE_ENV: &str = "CHEENHUB_DIOXUS_PROFILE";
    const FILTER_ENV: &str = "CHEENHUB_DIOXUS_PROFILE_FILTER";
    const LOG_ENV: &str = "CHEENHUB_DIOXUS_PROFILE_LOG";
    const DEFAULT_FILTER: &str = concat!(
        "info,",
        "dioxus_core::scope_arena=trace,",
        "dioxus_core::virtual_dom=trace,",
        "dioxus_core::runtime=trace,",
        "cheen_hub::features::voice_chat=trace,",
        "cheen_hub::features::social=trace",
    );

    if std::env::var_os(ENABLE_ENV).is_none() {
        return;
    }
    if dispatcher::has_been_set() {
        eprintln!("{ENABLE_ENV}=1 не включил Dioxus-профилирование: tracing subscriber уже задан.");
        return;
    }

    let log_path = std::env::var_os(LOG_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(default_dioxus_profile_log_path);
    if let Some(parent) = log_path.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        eprintln!(
            "{ENABLE_ENV}=1 не включил Dioxus-профилирование: не удалось создать каталог {}: {error}.",
            parent.display()
        );
        return;
    }
    let log_file = match std::fs::File::create(&log_path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!(
                "{ENABLE_ENV}=1 не включил Dioxus-профилирование: не удалось открыть {}: {error}.",
                log_path.display()
            );
            return;
        }
    };

    let filter_text = std::env::var(FILTER_ENV).unwrap_or_else(|_| DEFAULT_FILTER.to_owned());
    let filter = EnvFilter::try_new(&filter_text).unwrap_or_else(|error| {
        eprintln!(
            "{FILTER_ENV}='{filter_text}' не разобран ({error}); используется {DEFAULT_FILTER}."
        );
        EnvFilter::new(DEFAULT_FILTER)
    });
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(filter)
        .with_span_events(FmtSpan::CLOSE)
        .with_target(true)
        .with_writer(move || {
            log_file
                .try_clone()
                .expect("Dioxus profile log file handle can be cloned")
        })
        .finish();

    if set_global_default(subscriber).is_err() {
        eprintln!("{ENABLE_ENV}=1 не включил Dioxus-профилирование: subscriber отклонен.");
        return;
    }

    eprintln!("Dioxus profile log: {}", log_path.display());
    info!(
        filter = %filter_text,
        log_path = %log_path.display(),
        "Dioxus profile logging enabled; render spans will include close timing"
    );
}

#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
fn default_dioxus_profile_log_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("dioxus-profile.log")
}

#[cfg(not(all(feature = "desktop", not(target_arch = "wasm32"))))]
pub(super) fn launch_client(app: fn() -> dioxus::prelude::Element) {
    super::web::launch_client(app);
}

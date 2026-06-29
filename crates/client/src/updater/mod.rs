//! Окно и сценарий установки скачанного обновления.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use dioxus::prelude::*;

mod platform;

static TAILWIND_CSS: Asset = asset!(
    "/assets/tailwind.css",
    AssetOptions::css().with_static_head(true)
);
static CONFIG: OnceLock<UpdaterConfig> = OnceLock::new();

#[derive(Clone, Debug)]
struct UpdaterConfig {
    installer_path: PathBuf,
    app_pid: Option<u32>,
    restart_path: Option<PathBuf>,
    version: Option<String>,
    log_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct UpdaterState {
    stage: UpdaterStage,
    detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UpdaterStage {
    Preparing,
    WaitingForApp,
    Installing,
    Restarting,
    Complete,
    Failed,
}

/// Запускает отдельное окно апдейтера.
pub(crate) fn run() {
    let config = match parse_config() {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{message}");
            return;
        }
    };

    write_log(&config, "updater started");
    let _ = CONFIG.set(config);

    use dioxus::desktop::{Config, LogicalSize, WindowBuilder, icon_from_memory};

    let icon = icon_from_memory(include_bytes!(
        "../../../../crates/client/public/icons/icon-512.png"
    ))
    .expect("failed to load updater window icon");
    let window = WindowBuilder::new()
        .with_title("Обновление CheenHub")
        .with_inner_size(LogicalSize::new(520.0, 420.0))
        .with_resizable(false);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(window)
                .with_icon(icon)
                .with_menu(None),
        )
        .launch(UpdaterApp);
}

#[component]
fn UpdaterApp() -> Element {
    let mut started = use_signal(|| false);
    let state = use_signal(|| UpdaterState {
        stage: UpdaterStage::Preparing,
        detail: "Подготовка пакета обновления.".to_owned(),
    });

    use_effect(move || {
        if started() {
            return;
        }

        started.set(true);
        let config = CONFIG
            .get()
            .expect("updater config must be initialized before launch")
            .clone();
        spawn(async move {
            run_installation(config, state).await;
        });
    });

    let state_value = state();
    let progress = stage_progress(state_value.stage);
    let version = CONFIG
        .get()
        .and_then(|config| config.version.as_deref())
        .unwrap_or("новой версии");
    let title = stage_title(state_value.stage);
    let is_failed = state_value.stage == UpdaterStage::Failed;
    let is_complete = state_value.stage == UpdaterStage::Complete;

    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        main { class: "grid h-screen place-items-center overflow-hidden bg-zinc-950 px-6 py-6 text-zinc-100 selection:bg-zinc-700/40",
            div { class: "grid-bg absolute inset-0" }
            section { class: "relative w-full max-w-[420px] rounded-2xl border border-zinc-800 bg-zinc-950/90 p-5 shadow-[0_30px_110px_rgba(0,0,0,.65)]",
                div { class: "mb-3 flex h-11 w-11 items-center justify-center rounded-xl bg-accent text-[14px] font-bold text-white shadow-[0_14px_36px_rgba(59,130,246,.20)]", "CH" }
                div { class: "text-[11px] font-semibold uppercase tracking-[0.22em] text-zinc-500", "CheenHub" }
                h1 { class: "mt-2 text-[22px] font-semibold tracking-[-0.04em] text-zinc-50", "Обновление до {version}" }
                div { class: status_class(is_failed, is_complete),
                    div { class: pulse_class(is_failed, is_complete) }
                    div {
                        strong { class: "block text-[13px] font-semibold leading-5 text-zinc-100", "{title}" }
                        span { class: "mt-1 block text-[12px] leading-5 text-zinc-400", "{state_value.detail}" }
                    }
                }
                div { class: "mt-5 h-2 overflow-hidden rounded-full bg-zinc-900",
                    div {
                        class: progress_fill_class(is_failed),
                        style: "width: {progress}%;"
                    }
                }
                div { class: "mt-3 flex items-center justify-between gap-3",
                    span { class: "text-[12px] font-medium text-zinc-500", "{progress}%" }
                    span { class: "text-[12px] text-zinc-500", "Завершится автоматически" }
                }
                if is_complete {
                    button {
                        class: primary_button_class(false),
                        onclick: move |_| {
                            exit_updater(0);
                        },
                        "Готово"
                    }
                } else if is_failed {
                    button {
                        class: primary_button_class(true),
                        onclick: move |_| {
                            exit_updater(1);
                        },
                        "Закрыть"
                    }
                }
            }
        }
    }
}

fn status_class(is_failed: bool, is_complete: bool) -> &'static str {
    if is_failed {
        "mt-5 flex min-h-14 items-center gap-3 rounded-xl border border-red-500/20 bg-red-500/10 p-3"
    } else if is_complete {
        "mt-5 flex min-h-14 items-center gap-3 rounded-xl border border-emerald-500/20 bg-emerald-500/10 p-3"
    } else {
        "mt-5 flex min-h-14 items-center gap-3 rounded-xl border border-zinc-800 bg-zinc-900/60 p-3"
    }
}

fn pulse_class(is_failed: bool, is_complete: bool) -> &'static str {
    if is_failed {
        "h-2.5 w-2.5 shrink-0 rounded-full bg-red-400"
    } else if is_complete {
        "h-2.5 w-2.5 shrink-0 rounded-full bg-emerald-300"
    } else {
        "h-2.5 w-2.5 shrink-0 animate-pulse rounded-full bg-blue-500 shadow-[0_0_18px_rgba(96,165,250,0.55)]"
    }
}

fn progress_fill_class(is_failed: bool) -> &'static str {
    if is_failed {
        "h-full rounded-full bg-red-500 transition-[width] duration-100"
    } else {
        "h-full rounded-full bg-blue-500 transition-[width] duration-100"
    }
}

fn primary_button_class(is_danger: bool) -> &'static str {
    if is_danger {
        "mt-5 flex h-10 items-center justify-center rounded-xl border border-red-500/30 bg-red-500/15 px-4 text-[13px] font-semibold text-red-100 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-red-500/40 hover:bg-red-500/15"
    } else {
        "mt-5 flex h-10 items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)] transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-blue-400"
    }
}

async fn run_installation(config: UpdaterConfig, mut state: Signal<UpdaterState>) {
    set_stage(
        &mut state,
        UpdaterStage::Preparing,
        "Проверяем установочный пакет.",
    );
    sleep_step().await;

    if !config.installer_path.is_file() {
        fail(
            &config,
            &mut state,
            format!(
                "Файл обновления не найден: {}.",
                config.installer_path.display()
            ),
        );
        return;
    }

    if let Some(pid) = config.app_pid {
        set_stage(
            &mut state,
            UpdaterStage::WaitingForApp,
            "Ожидаем завершения текущего процесса.",
        );
        write_log(&config, &format!("waiting for app pid {pid}"));
        if let Err(error) = tokio::task::spawn_blocking(move || {
            platform::wait_for_process_exit(pid);
        })
        .await
        {
            fail(
                &config,
                &mut state,
                format!("Не удалось дождаться закрытия CheenHub: {error}"),
            );
            return;
        }
    }

    set_stage(
        &mut state,
        UpdaterStage::Installing,
        "Применяем обновление.",
    );
    sleep_step().await;
    write_log(
        &config,
        &format!("starting installer {}", config.installer_path.display()),
    );

    let installer_path = config.installer_path.clone();
    let installer_log_config = config.clone();
    match tokio::task::spawn_blocking(move || {
        platform::run_installer(&installer_path, |message| {
            write_log(&installer_log_config, message);
        })
    })
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(message)) => {
            fail(&config, &mut state, message);
            return;
        }
        Err(error) => {
            fail(
                &config,
                &mut state,
                format!("Установщик обновления остановился неожиданно: {error}"),
            );
            return;
        }
    }

    if let Some(restart_path) = &config.restart_path {
        set_stage(
            &mut state,
            UpdaterStage::Restarting,
            "Запускаем CheenHub заново.",
        );
        sleep_step().await;
        if let Err(message) = platform::restart_application(restart_path) {
            fail(&config, &mut state, message);
            return;
        }
    }

    write_log(&config, "updater completed");
    set_stage(
        &mut state,
        UpdaterStage::Complete,
        "Обновление установлено.",
    );
    sleep_step().await;
    write_log(&config, "closing updater after successful installation");
    exit_updater(0);
}

fn set_stage(state: &mut Signal<UpdaterState>, stage: UpdaterStage, detail: impl Into<String>) {
    state.set(UpdaterState {
        stage,
        detail: detail.into(),
    });
}

fn fail(config: &UpdaterConfig, state: &mut Signal<UpdaterState>, message: String) {
    write_log(config, &format!("updater failed: {message}"));
    set_stage(state, UpdaterStage::Failed, message);
}

fn exit_updater(code: i32) {
    std::process::exit(code);
}

async fn sleep_step() {
    tokio::time::sleep(Duration::from_millis(450)).await;
}

fn stage_title(stage: UpdaterStage) -> &'static str {
    match stage {
        UpdaterStage::Preparing => "Подготовка",
        UpdaterStage::WaitingForApp => "Ожидание",
        UpdaterStage::Installing => "Установка",
        UpdaterStage::Restarting => "Перезапуск",
        UpdaterStage::Complete => "Готово",
        UpdaterStage::Failed => "Ошибка установки",
    }
}

fn stage_progress(stage: UpdaterStage) -> u8 {
    match stage {
        UpdaterStage::Preparing => 16,
        UpdaterStage::WaitingForApp => 34,
        UpdaterStage::Installing => 72,
        UpdaterStage::Restarting => 90,
        UpdaterStage::Complete | UpdaterStage::Failed => 100,
    }
}

fn parse_config() -> Result<UpdaterConfig, String> {
    let mut args = std::env::args().skip(1);
    let mut installer_path = None;
    let mut app_pid = None;
    let mut restart_path = None;
    let mut version = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--installer" => installer_path = args.next().map(PathBuf::from),
            "--app-pid" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Не передан PID основного приложения.".to_owned())?;
                app_pid = value.parse::<u32>().ok();
            }
            "--restart" => restart_path = args.next().map(PathBuf::from),
            "--version" => version = args.next(),
            _ => {}
        }
    }

    let installer_path =
        installer_path.ok_or_else(|| "Не передан путь к установщику обновления.".to_owned())?;
    let log_path = std::env::temp_dir().join("cheenhub-updater.log");

    Ok(UpdaterConfig {
        installer_path,
        app_pid,
        restart_path,
        version,
        log_path,
    })
}

fn write_log(config: &UpdaterConfig, message: &str) {
    use std::io::Write;

    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.log_path)
    else {
        eprintln!("{message}");
        return;
    };

    let _ = writeln!(file, "[cheenhub_update_mode] {message}");
}

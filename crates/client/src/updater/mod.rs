//! Окно и сценарий установки скачанного обновления.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use dioxus::prelude::*;

mod platform;

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
        .with_inner_size(LogicalSize::new(520.0, 360.0))
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
        detail: "Готовим установку обновления...".to_owned(),
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
        style { {UPDATER_STYLE} }
        main { class: "updater-shell",
            section { class: "updater-card",
                div { class: "updater-mark", "CH" }
                div { class: "updater-eyebrow", "CheenHub" }
                h1 { "Обновление до {version}" }
                div { class: if is_failed { "status failed" } else if is_complete { "status complete" } else { "status" },
                    div { class: "pulse" }
                    div {
                        strong { "{title}" }
                        span { "{state_value.detail}" }
                    }
                }
                div { class: "progress-track",
                    div {
                        class: if is_failed { "progress-fill failed" } else { "progress-fill" },
                        style: "width: {progress}%;"
                    }
                }
                div { class: "progress-caption",
                    span { "{progress}%" }
                    span { "Окно можно оставить открытым" }
                }
                if is_complete {
                    button {
                        class: "primary-button",
                        onclick: move |_| {
                            exit_updater(0);
                        },
                        "Готово"
                    }
                } else if is_failed {
                    button {
                        class: "primary-button danger",
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

async fn run_installation(config: UpdaterConfig, mut state: Signal<UpdaterState>) {
    set_stage(
        &mut state,
        UpdaterStage::Preparing,
        "Проверяем скачанный установщик и параметры запуска.",
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
            "Закрываем основное приложение перед установкой.",
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
        "Запускаем установщик и ждём завершения обновления.",
    );
    sleep_step().await;
    write_log(
        &config,
        &format!("starting installer {}", config.installer_path.display()),
    );

    let installer_path = config.installer_path.clone();
    match tokio::task::spawn_blocking(move || platform::run_installer(&installer_path)).await {
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
            "Возвращаем CheenHub после установки.",
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
        "Обновление завершено. Можно продолжить работу.",
    );
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
        UpdaterStage::WaitingForApp => "Закрываем CheenHub",
        UpdaterStage::Installing => "Устанавливаем обновление",
        UpdaterStage::Restarting => "Перезапускаем приложение",
        UpdaterStage::Complete => "Готово",
        UpdaterStage::Failed => "Нужна помощь",
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

    let _ = writeln!(file, "[cheenhub_updater] {message}");
}

const UPDATER_STYLE: &str = r#"
html, body, #main {
    width: 100%;
    height: 100%;
    margin: 0;
}

body {
    font-family: Inter, "Segoe UI", system-ui, sans-serif;
    background: #121316;
    color: #f5f7fb;
}

.updater-shell {
    align-items: center;
    background:
        radial-gradient(circle at 20% 20%, rgba(79, 150, 255, 0.24), transparent 30%),
        linear-gradient(145deg, #191b20, #0e1013 68%);
    box-sizing: border-box;
    display: flex;
    height: 100%;
    justify-content: center;
    padding: 28px;
}

.updater-card {
    width: min(420px, 100%);
}

.updater-mark {
    align-items: center;
    background: #f5f7fb;
    border-radius: 8px;
    color: #111318;
    display: flex;
    font-size: 15px;
    font-weight: 800;
    height: 46px;
    justify-content: center;
    letter-spacing: 0;
    margin-bottom: 22px;
    width: 46px;
}

.updater-eyebrow {
    color: #8fa1bd;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0;
    margin-bottom: 8px;
    text-transform: uppercase;
}

h1 {
    font-size: 28px;
    line-height: 1.12;
    margin: 0 0 24px;
}

.status {
    align-items: center;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    display: flex;
    gap: 14px;
    min-height: 72px;
    padding: 14px;
}

.status.complete {
    border-color: rgba(45, 212, 191, 0.44);
}

.status.failed {
    border-color: rgba(248, 113, 113, 0.52);
}

.pulse {
    animation: pulse 1.4s ease-in-out infinite;
    background: #6ea8ff;
    border-radius: 999px;
    box-shadow: 0 0 0 0 rgba(110, 168, 255, 0.42);
    flex: 0 0 auto;
    height: 12px;
    width: 12px;
}

.complete .pulse {
    animation: none;
    background: #2dd4bf;
}

.failed .pulse {
    animation: none;
    background: #f87171;
}

strong {
    display: block;
    font-size: 15px;
    margin-bottom: 5px;
}

span {
    color: #bac5d6;
    display: block;
    font-size: 13px;
    line-height: 1.45;
}

.progress-track {
    background: rgba(255, 255, 255, 0.08);
    border-radius: 999px;
    height: 9px;
    margin-top: 22px;
    overflow: hidden;
}

.progress-fill {
    background: linear-gradient(90deg, #6ea8ff, #2dd4bf);
    border-radius: inherit;
    height: 100%;
    transition: width 320ms ease;
}

.progress-fill.failed {
    background: #f87171;
}

.progress-caption {
    display: flex;
    justify-content: space-between;
    margin-top: 10px;
}

.progress-caption span {
    color: #8fa1bd;
    font-size: 12px;
}

.primary-button {
    background: #f5f7fb;
    border: 0;
    border-radius: 8px;
    color: #111318;
    cursor: pointer;
    font: inherit;
    font-size: 14px;
    font-weight: 700;
    margin-top: 24px;
    min-height: 40px;
    padding: 0 18px;
}

.primary-button.danger {
    background: #fecaca;
    color: #7f1d1d;
}

@keyframes pulse {
    0% { box-shadow: 0 0 0 0 rgba(110, 168, 255, 0.42); }
    70% { box-shadow: 0 0 0 12px rgba(110, 168, 255, 0); }
    100% { box-shadow: 0 0 0 0 rgba(110, 168, 255, 0); }
}
"#;

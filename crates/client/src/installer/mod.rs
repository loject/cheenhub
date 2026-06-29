//! Пользовательский UI-установщик, запускающий вложенный системный установщик.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use dioxus::prelude::*;

mod payload {
    include!(concat!(env!("OUT_DIR"), "/installer_payload.rs"));
}

static TAILWIND_CSS: Asset = asset!(
    "/assets/tailwind.css",
    AssetOptions::css().with_static_head(true)
);
static CONFIG: OnceLock<InstallerConfig> = OnceLock::new();

#[derive(Clone, Debug)]
struct InstallerConfig {
    silent: bool,
    payload_name: String,
    log_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InstallerState {
    stage: InstallerStage,
    detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InstallerStage {
    Ready,
    Extracting,
    Installing,
    Complete,
    Failed,
}

/// Запускает установщик в интерактивном или тихом режиме.
pub(crate) fn run() {
    let config = InstallerConfig {
        silent: has_silent_flag(),
        payload_name: payload::INSTALLER_PAYLOAD_NAME
            .unwrap_or("cheenhub-installer-payload.exe")
            .to_owned(),
        log_path: std::env::temp_dir().join("cheenhub-installer.log"),
    };

    if config.silent {
        let code = match run_installation_blocking(&config, |_| {}) {
            Ok(()) => 0,
            Err(message) => {
                write_log(&config, &format!("silent installer failed: {message}"));
                1
            }
        };
        std::process::exit(code);
    }

    write_log(&config, "installer started");
    let _ = CONFIG.set(config);

    use dioxus::desktop::{Config, LogicalSize, WindowBuilder, icon_from_memory};

    let icon = icon_from_memory(include_bytes!(
        "../../../../crates/client/public/icons/icon-512.png"
    ))
    .expect("failed to load installer window icon");
    let window = WindowBuilder::new()
        .with_title("Установка CheenHub")
        .with_inner_size(LogicalSize::new(560.0, 420.0))
        .with_resizable(false);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(window)
                .with_icon(icon)
                .with_menu(None),
        )
        .launch(InstallerApp);
}

#[component]
fn InstallerApp() -> Element {
    let state = use_signal(|| InstallerState {
        stage: InstallerStage::Ready,
        detail: "Установим CheenHub через проверенный системный установщик.".to_owned(),
    });

    let state_value = state();
    let progress = stage_progress(state_value.stage);
    let title = stage_title(state_value.stage);
    let is_ready = state_value.stage == InstallerStage::Ready;
    let is_working = matches!(
        state_value.stage,
        InstallerStage::Extracting | InstallerStage::Installing
    );
    let is_failed = state_value.stage == InstallerStage::Failed;
    let is_complete = state_value.stage == InstallerStage::Complete;

    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        main { class: "grid h-screen place-items-center overflow-hidden bg-zinc-950 px-6 py-8 text-zinc-100 selection:bg-zinc-700/40",
            div { class: "grid-bg absolute inset-0" }
            section { class: "relative w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-950/90 p-6 shadow-[0_30px_110px_rgba(0,0,0,.65)]",
                div { class: "mb-5 flex items-center gap-3",
                    div { class: "flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-accent text-[14px] font-bold text-white shadow-[0_14px_36px_rgba(59,130,246,.20)]", "CH" }
                    div {
                        div { class: "text-[14px] font-semibold text-zinc-50", "CheenHub" }
                        div { class: "mt-0.5 text-[12px] leading-5 text-zinc-500", "Голосовой чат для своих" }
                    }
                }
                h1 { class: "text-[22px] font-semibold tracking-[-0.04em] text-zinc-50", "Установка CheenHub" }
                p { class: "mt-2 text-[13px] leading-6 text-zinc-500", "Подготовим приложение и передадим установку штатному установщику CheenHub." }
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
                div { class: "mt-5 flex items-center justify-between gap-3",
                    span { class: "text-[12px] font-medium text-zinc-500", "{progress}%" }
                    if is_ready {
                        button {
                            class: primary_button_class(false),
                            onclick: move |_| start_installation(state),
                            "Установить"
                        }
                    } else if is_working {
                        button {
                            class: "flex h-10 cursor-wait items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/70 px-4 text-[12px] font-semibold text-zinc-500",
                            disabled: true,
                            "Устанавливаем..."
                        }
                    } else if is_complete {
                        button {
                            class: primary_button_class(false),
                            onclick: move |_| {
                                exit_installer(0);
                            },
                            "Готово"
                        }
                    } else {
                        button {
                            class: primary_button_class(true),
                            onclick: move |_| {
                                exit_installer(1);
                            },
                            "Закрыть"
                        }
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
        "flex h-10 items-center justify-center rounded-xl border border-red-500/30 bg-red-500/15 px-4 text-[13px] font-semibold text-red-100 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-red-500/40 hover:bg-red-500/15"
    } else {
        "flex h-10 items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)] transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-blue-400"
    }
}

fn start_installation(mut state: Signal<InstallerState>) {
    let Some(config) = CONFIG.get().cloned() else {
        state.set(InstallerState {
            stage: InstallerStage::Failed,
            detail: "Не удалось прочитать параметры установщика.".to_owned(),
        });
        return;
    };

    spawn(async move {
        set_stage(
            &mut state,
            InstallerStage::Extracting,
            "Подготавливаем встроенный установщик.",
        );
        tokio::time::sleep(Duration::from_millis(350)).await;
        set_stage(
            &mut state,
            InstallerStage::Installing,
            "Запускаем системный установщик CheenHub.",
        );

        let result = tokio::task::spawn_blocking(move || {
            run_installation_blocking(&config, |stage| {
                write_log(&config, stage);
            })
        })
        .await;

        match result {
            Ok(Ok(())) => {
                set_stage(
                    &mut state,
                    InstallerStage::Complete,
                    "Установка завершена. Можно закрыть это окно.",
                );
            }
            Ok(Err(message)) => {
                set_stage(&mut state, InstallerStage::Failed, message);
            }
            Err(error) => {
                set_stage(
                    &mut state,
                    InstallerStage::Failed,
                    format!("Установка остановилась неожиданно: {error}"),
                );
            }
        }
    });
}

fn run_installation_blocking(
    config: &InstallerConfig,
    mut on_stage: impl FnMut(&str),
) -> Result<(), String> {
    let Some(payload) = payload::INSTALLER_PAYLOAD else {
        return Err("В этот файл не встроен системный установщик CheenHub.".to_owned());
    };

    on_stage("extracting installer payload");
    let payload_path = write_payload(config, payload)?;
    on_stage("running nested installer payload");
    run_payload_installer(&payload_path)?;
    on_stage("nested installer completed");
    Ok(())
}

fn write_payload(config: &InstallerConfig, payload: &[u8]) -> Result<PathBuf, String> {
    let install_dir =
        std::env::temp_dir().join(format!("cheenhub-installer-{}", std::process::id()));
    std::fs::create_dir_all(&install_dir).map_err(|error| {
        format!(
            "Не удалось подготовить временную папку {}: {error}",
            install_dir.display()
        )
    })?;

    let payload_path = install_dir.join(&config.payload_name);
    std::fs::write(&payload_path, payload).map_err(|error| {
        format!(
            "Не удалось распаковать установщик в {}: {error}",
            payload_path.display()
        )
    })?;

    Ok(payload_path)
}

#[cfg(target_os = "windows")]
fn run_payload_installer(payload_path: &Path) -> Result<(), String> {
    let extension = payload_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut command = if extension == "msi" {
        let mut command = Command::new("msiexec");
        command
            .arg("/i")
            .arg(payload_path)
            .arg("/passive")
            .arg("/norestart");
        command
    } else {
        let mut command = Command::new(payload_path);
        command.arg("/S");
        command
    };

    let status = command
        .status()
        .map_err(|error| format!("Не удалось запустить системный установщик: {error}"))?;
    if status.success() {
        return Ok(());
    }

    Err(format!(
        "Системный установщик завершился с кодом {}.",
        status
            .code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| "unknown".to_owned())
    ))
}

#[cfg(not(target_os = "windows"))]
fn run_payload_installer(_payload_path: &Path) -> Result<(), String> {
    Err("Пользовательский установщик пока доступен только для Windows.".to_owned())
}

fn set_stage(state: &mut Signal<InstallerState>, stage: InstallerStage, detail: impl Into<String>) {
    state.set(InstallerState {
        stage,
        detail: detail.into(),
    });
}

fn stage_title(stage: InstallerStage) -> &'static str {
    match stage {
        InstallerStage::Ready => "Готово к установке",
        InstallerStage::Extracting => "Подготовка",
        InstallerStage::Installing => "Установка",
        InstallerStage::Complete => "Готово",
        InstallerStage::Failed => "Не удалось установить",
    }
}

fn stage_progress(stage: InstallerStage) -> u8 {
    match stage {
        InstallerStage::Ready => 8,
        InstallerStage::Extracting => 34,
        InstallerStage::Installing => 76,
        InstallerStage::Complete | InstallerStage::Failed => 100,
    }
}

fn has_silent_flag() -> bool {
    std::env::args().skip(1).any(|arg| {
        let arg = arg.to_ascii_lowercase();
        matches!(arg.as_str(), "/s" | "-s" | "--silent" | "/silent")
    })
}

fn exit_installer(code: i32) {
    std::process::exit(code);
}

fn write_log(config: &InstallerConfig, message: &str) {
    use std::io::Write;

    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.log_path)
    else {
        eprintln!("{message}");
        return;
    };

    let _ = writeln!(file, "[cheenhub_installer] {message}");
}

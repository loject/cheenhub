//! Статическая сборка FFmpeg, oneVPL и программного VP9 из архивов проекта.

mod steps;

use crate::{XtaskResult, checked_status};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

struct Build {
    vendor: PathBuf,
    directory: PathBuf,
    prefix: PathBuf,
    windows: bool,
    jobs: String,
}

/// Проверяет архивы и собирает статические зависимости desktop-видео без сети.
///
/// Ошибки проверки исходников или внешних инструментов останавливают сборку.
pub(super) fn run(args: Vec<String>) -> XtaskResult<()> {
    if !args.is_empty() {
        return Err("Использование: cargo run -p xtask -- build-ffmpeg".into());
    }
    let windows = match env::consts::OS {
        "windows" => true,
        "linux" => false,
        os => return Err(format!("FFmpeg backend не поддерживает платформу {os}")),
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("Нет корня workspace")?;
    let build = Build {
        vendor: root.join("crates/video-codec/vendor"),
        directory: root.join("target/ffmpeg-build"),
        prefix: root.join("target/ffmpeg-build/ffmpeg-static"),
        windows,
        jobs: env::var("NUM_JOBS").unwrap_or_else(|_| "4".into()),
    };
    let manifest = fs::read(build.vendor.join("SHA256SUMS")).map_err(|error| error.to_string())?;
    let manifest_text = std::str::from_utf8(&manifest).map_err(|error| error.to_string())?;
    let mut archives = Vec::new();
    for line in manifest_text.lines().filter(|line| !line.trim().is_empty()) {
        let (expected, name) = line.split_once("  ").ok_or("Некорректный SHA256SUMS")?;
        let path = build.vendor.join(name);
        let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        verify_bytes(&bytes, expected).map_err(|error| format!("{name}: {error}"))?;
        println!("[FFmpeg] Архив проверен: {name}");
        archives.push(path);
    }
    let stamp = fingerprint(&[
        &manifest,
        env::consts::OS.as_bytes(),
        env::consts::ARCH.as_bytes(),
        include_bytes!("mod.rs"),
        include_bytes!("steps.rs"),
    ]);
    let marker = build.prefix.join("complete");
    if fs::read_to_string(&marker).is_ok_and(|value| value == stamp) {
        println!("[FFmpeg] Статические библиотеки уже собраны.");
        return Ok(());
    }
    if marker.exists() {
        fs::remove_file(&marker).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(build.directory.join("sources")).map_err(|error| error.to_string())?;
    fs::create_dir_all(&build.prefix).map_err(|error| error.to_string())?;
    for archive in archives {
        let file = fs::File::open(&archive).map_err(|error| error.to_string())?;
        // stdin устраняет трактовку буквы Windows-диска как remote archive GNU tar.
        let format = if archive
            .extension()
            .is_some_and(|extension| extension == "xz")
        {
            "-xJf"
        } else {
            "-xzf"
        };
        execute(
            "распаковка исходников",
            Command::new("tar")
                .args([format, "-", "-C"])
                .arg(build.directory.join("sources"))
                .stdin(file),
        )?;
    }
    steps::vpx(&build)?;
    steps::vpl(&build)?;
    steps::ffmpeg(&build)?;
    for name in [
        "FFmpeg-LICENSE",
        "oneVPL-LICENSE",
        "oneVPL-THIRD-PARTY",
        "libvpx-LICENSE",
        "libvpx-PATENTS",
    ] {
        fs::copy(build.vendor.join(name), build.prefix.join(name))
            .map_err(|error| error.to_string())?;
    }
    // ffmpeg-sys-next упаковывает статические архивы в rlib и не отслеживает
    // их замену внутри FFMPEG_DIR. Удаляем только его воспроизводимый Cargo-кэш.
    execute(
        "обновление Cargo-кэша FFmpeg",
        Command::new("cargo")
            .args(["clean", "--offline", "-p", "ffmpeg-sys-next"])
            .current_dir(root),
    )?;
    fs::write(marker, stamp).map_err(|error| error.to_string())?;
    println!(
        "[FFmpeg] Статическая сборка завершена: {}",
        build.prefix.display()
    );
    Ok(())
}

fn verify_bytes(bytes: &[u8], expected: &str) -> XtaskResult<()> {
    let actual = hex_digest(bytes);
    if actual != expected {
        return Err(format!("SHA256 не совпадает: {actual}"));
    }
    Ok(())
}

fn fingerprint(parts: &[&[u8]]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_le_bytes());
        digest.update(part);
    }
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn execute(stage: &str, command: &mut Command) -> XtaskResult<()> {
    println!("[FFmpeg] {stage}");
    checked_status(command).map_err(|error| format!("FFmpeg, {stage}: {error}"))
}

impl Build {
    fn source(&self, name: &str) -> PathBuf {
        self.directory.join("sources").join(name)
    }

    fn posix_path(&self, path: &Path) -> XtaskResult<String> {
        if !self.windows {
            return Ok(path.to_string_lossy().into_owned());
        }
        let result = Command::new("cygpath")
            .arg("-u")
            .arg(path)
            .output()
            .map_err(|error| format!("Нужны MSYS2 и cygpath: {error}"))?;
        if !result.status.success() {
            return Err("cygpath не преобразовал Windows-путь".into());
        }
        Ok(String::from_utf8_lossy(&result.stdout).trim().to_owned())
    }

    fn configure(&self, source: &str, output: &str) -> XtaskResult<Command> {
        let directory = self.directory.join(output);
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let mut command = Command::new("sh");
        command
            .arg(self.posix_path(&self.source(source).join("configure"))?)
            .current_dir(directory);
        let prefix = self.posix_path(&self.prefix)?;
        let existing = env::var("PKG_CONFIG_PATH").unwrap_or_default();
        command.env(
            "PKG_CONFIG_PATH",
            format!(
                "{prefix}/lib/pkgconfig{}",
                if existing.is_empty() {
                    String::new()
                } else {
                    format!(":{existing}")
                }
            ),
        );
        Ok(command)
    }
}

#[cfg(test)]
mod tests;

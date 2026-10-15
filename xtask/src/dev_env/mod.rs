//! Локальная подготовка PostgreSQL и JWT без изменения действующих секретов.

mod database;
mod env_file;
mod jwt;

use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::XtaskResult;
use env_file::EnvFile;

/// Выполняет `dev-env ensure`, сохраняя существующие секреты и конфигурацию.
///
/// При ошибке проверки исходный файл не переписывается. Аргумент `--file`
/// позволяет проверить подготовку на отдельной конфигурации.
pub(super) fn run(args: Vec<String>) -> XtaskResult<()> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?;
    let mut args = args.into_iter();
    if args.next().as_deref() != Some("ensure") {
        return Err("usage: dev-env ensure [--file <path>]".into());
    }
    let path = match args.next() {
        None => repo.join(".env"),
        Some(flag) if flag == "--file" => {
            PathBuf::from(args.next().ok_or("--file requires an explicit path")?)
        }
        Some(_) => return Err("usage: dev-env ensure [--file <path>]".into()),
    };
    if args.next().is_some() {
        return Err("unexpected argument for dev-env ensure".into());
    }
    ensure_file(&path, &repo.join(".env.example"))
}

fn ensure_file(path: &Path, template: &Path) -> XtaskResult<()> {
    let (original, exists) = match fs::read_to_string(path) {
        Ok(content) => (content, true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (
            fs::read_to_string(template)
                .map_err(|error| format!("cannot read {}: {error}", template.display()))?,
            false,
        ),
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };

    let mut env = EnvFile::new(original.clone());
    if !exists {
        // Опубликованные примерные секреты не переносятся в новую .env.
        for key in [
            "POSTGRES_PASSWORD",
            "DATABASE_URL",
            "JWT_ED25519_PRIVATE_KEY_BASE64",
            "CHEENHUB_JWT_PUBLIC_KEY_BASE64",
        ] {
            env.remove(key)?;
        }
    }
    database::ensure(&mut env)?;
    jwt::ensure(&mut env)?;

    if exists && env.content() == original {
        println!("Dev environment verified: {} (no changes)", path.display());
        return Ok(());
    }

    write_atomically(path, env.content())?;
    println!("Dev environment prepared: {}", path.display());
    Ok(())
}

fn write_atomically(path: &Path, content: &str) -> XtaskResult<()> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let file_name = path.file_name().ok_or("invalid .env file path")?;
    let temporary = parent.join(format!(
        ".{}.tmp-{}-{nonce}",
        file_name.to_string_lossy(),
        std::process::id()
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| -> XtaskResult<()> {
        let mut file = options
            .open(&temporary)
            .map_err(|error| format!("cannot create temporary .env: {error}"))?;
        file.write_all(content.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("cannot save temporary .env: {error}"))?;
        drop(file);
        fs::rename(&temporary, path)
            .map_err(|error| format!("cannot replace {}: {error}", path.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests;

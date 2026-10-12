//! Формирование статического HTML-артефакта лендинга для текущей web-сборки.

use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::XtaskResult;

/// Подставляет build-time версию в HTML-шаблон и записывает готовый артефакт.
///
/// Принимает необязательный путь выходного файла; версия обязательна в
/// `CHEENHUB_APP_VERSION`. Ошибка проверки не изменяет предыдущий артефакт.
pub(super) fn run(args: &[String]) -> XtaskResult<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("Не найден корень проекта")?;
    let default_output = root.join("target/dx/cheenhub/release/web/public/landing.html");
    let output = match args {
        [] => default_output.as_path(),
        [path] => Path::new(path),
        _ => return Err("Использование: release-artifacts render-landing [<output-html>]".into()),
    };
    let version = env::var("CHEENHUB_APP_VERSION").map_err(|_| {
        "[landing] Для формирования лендинга требуется CHEENHUB_APP_VERSION".to_owned()
    })?;
    let template = fs::read_to_string(root.join("crates/client/landing.html.template"))
        .map_err(|error| format!("[landing] Не удалось прочитать HTML-шаблон: {error}"))?;
    write_landing(output, &template, &version)?;
    println!(
        "[landing] HTML записан в {}, версия {version}",
        output.display()
    );
    Ok(())
}

fn render(template: &str, version: &str) -> XtaskResult<String> {
    let bare = version.strip_prefix('v').unwrap_or(version);
    let core = bare.split(['-', '+']).next().unwrap_or_default();
    let parts: Vec<_> = core.split('.').collect();
    let valid_core = parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
    let suffix = &bare[core.len()..];
    let valid_suffix = suffix.is_empty()
        || (suffix.len() > 1
            && suffix[1..].bytes().any(|byte| byte.is_ascii_alphanumeric())
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b".-+".contains(&byte)));
    if !valid_core || !valid_suffix {
        return Err("[landing] Некорректная CHEENHUB_APP_VERSION: ожидается версия major.minor.patch с необязательным суффиксом".into());
    }
    if !template.contains("@CHEENHUB_RELEASE_VERSION@") {
        return Err("[landing] В HTML-шаблоне отсутствует маркер версии".into());
    }
    let html = template.replace("@CHEENHUB_RELEASE_VERSION@", &format!("v{bare}"));
    if html.contains("@CHEENHUB_") {
        return Err("[landing] В HTML остались незаменённые маркеры".into());
    }
    Ok(html)
}

fn write_landing(output: &Path, template: &str, version: &str) -> XtaskResult<()> {
    let html = render(template, version)?;
    let temporary = output.with_extension(format!(
        "html.{}-{}.tmp",
        std::process::id(),
        unique_suffix()
    ));
    let result = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(html.as_bytes())?;
        drop(file);
        fs::rename(&temporary, output)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| {
        format!(
            "[landing] Не удалось записать {}: {error}",
            output.display()
        )
    })
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(test)]
mod tests;

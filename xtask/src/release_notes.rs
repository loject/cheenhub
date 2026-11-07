//! Проверка согласованного описания релиза и подготовка текстов для издателей.

use std::fs;
use std::path::{Path, PathBuf};

use crate::XtaskResult;

/// Тексты одной версии, проверенные перед публикацией артефактов.
pub(crate) struct ReleaseNotes {
    /// Полное описание для GitHub Release в Markdown.
    pub(crate) github: String,
    /// Описание изменений Android для поля «Что нового» в RuStore.
    pub(crate) rustore: String,
}

impl ReleaseNotes {
    /// Читает описание точного тега из `releases` относительно корня репозитория.
    ///
    /// Отсутствующий файл, неверный формат или превышение лимита блокируют выпуск.
    pub(crate) fn read(tag: &str) -> XtaskResult<Self> {
        let path = notes_path(tag)?;
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read release notes {}: {error}", path.display()))?;
        Self::parse(tag, &contents)
            .map_err(|error| format!("invalid release notes {}: {error}", path.display()))
    }

    fn parse(tag: &str, contents: &str) -> XtaskResult<Self> {
        let mut lines = contents.lines();
        if lines.next() != Some(format!("# {tag}").as_str()) {
            return Err(format!("first line must be # {tag}"));
        }

        let mut github = Vec::new();
        let mut rustore = Vec::new();
        let mut section = None;
        let mut seen = Vec::new();
        for line in lines {
            if let Some(heading) = line.strip_prefix("## ") {
                if !matches!(heading, "GitHub" | "RuStore") || seen.contains(&heading) {
                    return Err(format!("unknown or duplicate section: {heading}"));
                }
                seen.push(heading);
                section = Some(heading);
            } else {
                match section {
                    Some("GitHub") => github.push(line),
                    Some("RuStore") => rustore.push(line),
                    _ if line.trim().is_empty() => {}
                    _ => return Err("text must be inside GitHub or RuStore section".to_owned()),
                }
            }
        }

        let notes = Self {
            github: github.join("\n").trim().to_owned(),
            rustore: rustore.join("\n").trim().to_owned(),
        };
        if notes.github.is_empty() || notes.rustore.is_empty() {
            return Err("GitHub and RuStore descriptions must both be nonempty".to_owned());
        }
        let length = notes.rustore.chars().count();
        if length > 5000 {
            return Err(format!(
                "RuStore description has {length} characters; maximum is 5000"
            ));
        }
        Ok(notes)
    }
}

/// Проверяет описание или экспортирует проверенные тексты в указанный каталог.
///
/// Команда работает из корня репозитория и не обращается к внешним API.
pub(crate) fn run(args: Vec<String>) -> XtaskResult<()> {
    match args.as_slice() {
        [action, tag] if action == "check" => {
            ReleaseNotes::read(tag)?;
            println!("Release notes validated: {tag}");
            Ok(())
        }
        [action, tag, directory] if action == "export" => {
            let notes = ReleaseNotes::read(tag)?;
            let directory = Path::new(directory);
            fs::create_dir_all(directory)
                .map_err(|error| format!("failed to create {}: {error}", directory.display()))?;
            for (name, text) in [("github.md", notes.github), ("rustore.txt", notes.rustore)] {
                let path = directory.join(name);
                fs::write(&path, text)
                    .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
            }
            println!(
                "Release notes exported: tag={tag} directory={}",
                directory.display()
            );
            Ok(())
        }
        _ => Err(
            "Usage: cargo run -p xtask -- release-notes <check <tag>|export <tag> <directory>>"
                .to_owned(),
        ),
    }
}

fn notes_path(tag: &str) -> XtaskResult<PathBuf> {
    if !tag.starts_with('v')
        || tag.len() < 2
        || !tag[1..].chars().all(|character| {
            character.is_ascii_digit() || matches!(character, '.' | '-' | 'r' | 'c')
        })
        || !tag.as_bytes()[1].is_ascii_digit()
    {
        return Err(format!("invalid release notes tag: {tag:?}"));
    }
    Ok(Path::new("releases").join(format!("{tag}.md")))
}

#[cfg(test)]
mod tests;

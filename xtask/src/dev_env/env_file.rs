//! Чтение и точечное обновление локального dotenv-файла без потери комментариев.

use crate::XtaskResult;

/// Редактируемое содержимое файла переменных окружения.
pub(super) struct EnvFile {
    content: String,
}

impl EnvFile {
    /// Создаёт редактор для исходного содержимого файла.
    pub(super) fn new(content: String) -> Self {
        Self { content }
    }

    /// Возвращает единственное активное значение или ошибку при повторении ключа.
    pub(super) fn get(&self, key: &str) -> XtaskResult<Option<String>> {
        let mut found = None;
        for line in self.content.lines() {
            let Some((name, value)) = entry(line) else {
                continue;
            };
            if name != key {
                continue;
            }
            if found.is_some() {
                return Err(format!("duplicate {key} in .env; resolve manually"));
            }
            found = Some(unquote(value.trim()).to_owned());
        }
        Ok(found)
    }

    /// Возвращает непустое значение, считая пустую настройку отсутствующей.
    pub(super) fn nonempty(&self, key: &str) -> XtaskResult<Option<String>> {
        Ok(self.get(key)?.filter(|value| !value.trim().is_empty()))
    }

    /// Добавляет или обновляет один ключ, оставляя остальные строки неизменными.
    pub(super) fn set(&mut self, key: &str, value: &str) -> XtaskResult<()> {
        if value.contains(['\r', '\n']) {
            return Err(format!("invalid multiline value for {key}"));
        }
        // Проверяем дубликаты до редактирования, чтобы не скрыть конфликт.
        self.get(key)?;
        let mut lines = self.content.lines().map(str::to_owned).collect::<Vec<_>>();
        if let Some(line) = lines
            .iter_mut()
            .find(|line| entry(line).is_some_and(|(name, _)| name == key))
        {
            *line = format!("{key}={value}");
        } else {
            lines.push(format!("{key}={value}"));
        }
        self.content = format!("{}\n", lines.join("\n"));
        Ok(())
    }

    /// Удаляет активную настройку, не затрагивая комментарии и остальные ключи.
    pub(super) fn remove(&mut self, key: &str) -> XtaskResult<()> {
        self.get(key)?;
        let retained = self
            .content
            .lines()
            .filter(|line| entry(line).is_none_or(|(name, _)| name != key))
            .collect::<Vec<_>>();
        let trailing_newline = self.content.ends_with('\n');
        self.content = retained.join("\n");
        if trailing_newline && !self.content.is_empty() {
            self.content.push('\n');
        }
        Ok(())
    }

    /// Возвращает итоговое содержимое для записи на диск.
    pub(super) fn content(&self) -> &str {
        &self.content
    }
}

fn entry(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let line = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let (key, value) = line.split_once('=')?;
    Some((key.trim(), value))
}

fn unquote(value: &str) -> &str {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

#[cfg(test)]
mod tests;

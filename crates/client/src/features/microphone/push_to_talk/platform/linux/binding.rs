//! Linux-настройка хранит описание портала; сочетание принадлежит системному shortcut ID.

use std::sync::Mutex;

static REVISION: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(1);
static LABEL: Mutex<String> = Mutex::new(String::new());

/// Изолирует тесты единственного process-wide назначения; runtime этим lock не пользуется.
#[cfg(test)]
pub(super) static TEST_LOCK: Mutex<()> = Mutex::new(());

/// Ревизия назначения портала, независимая от Windows Virtual-Key.
///
/// Точная комбинация и разрешение сохраняются desktop portal под app ID и shortcut ID.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PushToTalkKey(u16);

impl PushToTalkKey {
    /// Загружает Linux-ревизию и описание; числовые Windows-настройки игнорируются.
    pub(crate) fn from_value(value: &str) -> Self {
        let Some(value) = value.strip_prefix("portal:") else {
            return Self::default();
        };
        let Some((revision, label)) = value.split_once(':') else {
            return Self::default();
        };
        let Ok(revision) = revision.parse::<u16>() else {
            return Self::default();
        };
        REVISION.fetch_max(
            revision.wrapping_add(1),
            std::sync::atomic::Ordering::Relaxed,
        );
        set_label(label);
        Self(revision)
    }

    /// Возвращает ревизию для диагностики; значение не является кодом клавиши.
    pub(crate) fn code(self) -> u16 {
        self.0
    }

    /// Возвращает отдельный формат Linux-настройки и последнее описание портала.
    pub(crate) fn storage_value(self) -> String {
        format!("portal:{}:{}", self.0, raw_label())
    }

    /// Возвращает описание сочетания, полученное от desktop portal.
    pub(crate) fn label(self) -> String {
        label()
    }

    /// Меняет ревизию после завершения системного назначения.
    pub(super) fn assigned() -> Self {
        Self(REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

/// Обновляет описание по BindShortcuts или ShortcutsChanged.
pub(super) fn set_label(value: &str) {
    *LABEL.lock().unwrap_or_else(|error| error.into_inner()) = value.to_owned();
}

/// Даёт читаемое пустое состояние до первого назначения.
pub(super) fn label() -> String {
    let label = LABEL.lock().unwrap_or_else(|error| error.into_inner());
    if label.is_empty() {
        "Сочетание не назначено".into()
    } else {
        label.clone()
    }
}

/// Возвращает подтверждённое описание без UI-подстановки для пустой привязки.
pub(super) fn raw_label() -> String {
    LABEL
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

#[cfg(test)]
mod tests;

//! Привязка к одной кнопке клавиатуры или мыши; числовой формат сохраняет прежние настройки.

/// Кнопка глобальной активации, представленная Windows virtual-key code.
///
/// Поддерживает одиночное удержание клавиш и пяти стандартных кнопок мыши.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PushToTalkKey(u16);

impl Default for PushToTalkKey {
    fn default() -> Self {
        Self(0xa3)
    }
}
impl PushToTalkKey {
    /// Проверяет код кнопки; нулевой и выходящий за диапазон код не принимается.
    pub(crate) fn from_code(code: u16) -> Option<Self> {
        (1..=254).contains(&code).then_some(Self(code))
    }

    /// Восстанавливает привязку; повреждённые значения заменяются правым Ctrl.
    pub(crate) fn from_value(value: &str) -> Self {
        value
            .parse()
            .ok()
            .and_then(Self::from_code)
            .unwrap_or_default()
    }

    /// Возвращает стабильный код для хранения и платформенного адаптера.
    pub(crate) fn code(self) -> u16 {
        self.0
    }

    /// Отличает кнопки мыши, для которых нужен отдельный low-level hook.
    pub(crate) fn is_mouse(self) -> bool {
        matches!(self.0, 1 | 2 | 4 | 5 | 6)
    }

    /// Возвращает название без зависимости UI от Windows API.
    pub(crate) fn label(self) -> String {
        super::platform::key_label(self)
    }

    /// Даёт читаемое название при отсутствии системного названия клавиши.
    pub(super) fn fallback_label(self) -> String {
        if self.is_mouse() {
            return match self.0 {
                1 => "Mouse1",
                2 => "Mouse2",
                4 => "Mouse3",
                5 => "Mouse4",
                _ => "Mouse5",
            }
            .into();
        }
        match self.0 {
            8 => "Backspace".into(),
            9 => "Tab".into(),
            13 => "Enter".into(),
            27 => "Escape".into(),
            32 => "Пробел".into(),
            0xa0 => "Левый Shift".into(),
            0xa1 => "Правый Shift".into(),
            0xa2 => "Левый Ctrl".into(),
            0xa3 => "Правый Ctrl".into(),
            0xa4 => "Левый Alt".into(),
            0xa5 => "Правый Alt".into(),
            0x30..=0x39 | 0x41..=0x5a => {
                char::from_u32(u32::from(self.0)).unwrap_or(' ').to_string()
            }
            0x70..=0x87 => format!("F{}", self.0 - 0x70 + 1),
            _ => "Дополнительная клавиша".into(),
        }
    }
}

#[cfg(test)]
mod tests;

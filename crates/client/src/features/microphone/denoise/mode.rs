//! Сохраняемый выбор алгоритма подавления шума микрофона.

/// Режим обработки звука, выбранный пользователем в настройках микрофона.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum DenoiseMode {
    /// Передаёт звук без подавления фонового шума.
    #[default]
    Off,
    /// Использует RNNoise через native-процессор nnnoiseless.
    Rnnoise,
}

impl DenoiseMode {
    /// Возвращает стабильное значение для хранения настройки и select.
    pub(crate) fn value(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Rnnoise => "rnnoise",
        }
    }

    /// Разбирает сохранённый режим; неизвестное значение отключает обработку.
    pub(crate) fn from_value(value: &str) -> Self {
        match value {
            "rnnoise" => Self::Rnnoise,
            _ => Self::Off,
        }
    }
}

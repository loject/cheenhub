//! Сохраняемый выбор алгоритма подавления шума микрофона.

/// Режим обработки звука, выбранный пользователем в настройках микрофона.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum DenoiseMode {
    /// Передаёт звук без подавления фонового шума.
    #[default]
    Off,
    /// Использует встроенное шумоподавление браузера без VAD RNNoise.
    Browser,
    /// Использует RNNoise через общий процессор nnnoiseless.
    Rnnoise,
}

impl DenoiseMode {
    /// Заменяет недоступный алгоритм платформенным вариантом без ложного включения DSP.
    pub(crate) fn supported(self, rnnoise: bool, browser: bool) -> Self {
        match self {
            Self::Rnnoise if !rnnoise => {
                if browser {
                    Self::Browser
                } else {
                    Self::Off
                }
            }
            Self::Browser if !browser => Self::Off,
            mode => mode,
        }
    }

    /// Возвращает стабильное значение для хранения настройки и select.
    pub(crate) fn value(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Browser => "browser",
            Self::Rnnoise => "rnnoise",
        }
    }

    /// Разбирает сохранённый режим; неизвестное значение отключает обработку.
    pub(crate) fn from_value(value: &str) -> Self {
        match value {
            "rnnoise" => Self::Rnnoise,
            "browser" => Self::Browser,
            _ => Self::Off,
        }
    }
    /// Разрешает настройки: новый ключ, прежний переключатель, затем платформенный дефолт.
    pub(crate) fn from_preferences(
        saved: Option<&str>,
        legacy: Option<&str>,
        default: Self,
    ) -> Self {
        match (saved, legacy) {
            (Some(value), _) => Self::from_value(value),
            (None, Some("true")) => Self::Rnnoise,
            (None, Some(_)) => Self::Off,
            (None, None) => default,
        }
    }
}

#[cfg(test)]
mod tests;

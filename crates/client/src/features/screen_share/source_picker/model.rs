//! Чистая модель выбора источника демонстрации экрана.

use cheenhub_contracts::video_presets::SCREEN_SHARE_BITRATE_LIMIT;

/// Экран, доступный для демонстрации.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScreenShareSource {
    /// Стабильный идентификатор экрана в пределах текущего списка.
    pub(crate) id: String,
    /// Отображаемое имя экрана.
    pub(crate) name: String,
    /// Ширина экрана в физических пикселях.
    pub(crate) width: u32,
    /// Высота экрана в физических пикселях.
    pub(crate) height: u32,
    /// Готовый URL изображения для предварительного просмотра.
    pub(crate) preview_url: String,
    /// Является ли экран основным.
    pub(crate) is_primary: bool,
}

/// Разрешение демонстрации экрана.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ScreenShareResolution {
    /// Изображение 1280x720.
    #[default]
    P720,
    /// Изображение 1920x1080.
    P1080,
    /// Изображение 2560x1440.
    P1440,
    /// Изображение 3840x2160.
    Uhd4K,
}

impl ScreenShareResolution {
    /// Возвращает целевые размеры кадра для выбранного разрешения.
    pub(crate) const fn dimensions(self) -> (u32, u32) {
        match self {
            Self::P720 => (1_280, 720),
            Self::P1080 => (1_920, 1_080),
            Self::P1440 => (2_560, 1_440),
            Self::Uhd4K => (3_840, 2_160),
        }
    }

    /// Возвращает название для итогового описания.
    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::P720 => "720p",
            Self::P1080 => "1080p",
            Self::P1440 => "1440p",
            Self::Uhd4K => "4K",
        }
    }
}

/// Частота кадров демонстрации экрана.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ScreenShareFrameRate {
    /// До 15 кадров в секунду.
    Fps15,
    /// До 24 кадров в секунду.
    Fps24,
    /// До 30 кадров в секунду.
    #[default]
    Fps30,
    /// До 60 кадров в секунду.
    Fps60,
}

impl ScreenShareFrameRate {
    /// Возвращает целевую максимальную частоту кадров.
    pub(crate) const fn max_fps(self) -> u32 {
        match self {
            Self::Fps15 => 15,
            Self::Fps24 => 24,
            Self::Fps30 => 30,
            Self::Fps60 => 60,
        }
    }

    /// Возвращает название для итогового описания.
    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::Fps15 => "15 кадров/с",
            Self::Fps24 => "24 кадра/с",
            Self::Fps30 => "30 кадров/с",
            Self::Fps60 => "60 кадров/с",
        }
    }
}

/// Раздел выбора источника.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ScreenShareSourceTab {
    /// Полные экраны.
    #[default]
    Screens,
    /// Отдельные окна приложений.
    Windows,
}

impl ScreenShareSourceTab {
    /// Проверяет, можно ли подтвердить текущий выбор.
    pub(crate) const fn can_confirm(self, selection: Option<&ScreenShareSelection>) -> bool {
        matches!(self, Self::Screens) && selection.is_some()
    }
}

/// Состояние загрузки списка экранов.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScreenShareSourcePickerState {
    /// Источник будет выбран в системном окне браузера.
    Browser,
    /// Список экранов загружается.
    Loading,
    /// Список экранов успешно загружен, включая допустимый пустой список.
    Ready(Vec<ScreenShareSource>),
    /// Список экранов не удалось загрузить.
    Error(String),
}

/// Подтверждённые пользователем параметры демонстрации.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScreenShareSelection {
    /// Идентификатор монитора, либо `None` для системного выбора браузера.
    pub(crate) source_id: Option<String>,
    /// Выбранное разрешение.
    pub(crate) resolution: ScreenShareResolution,
    /// Выбранная частота кадров.
    pub(crate) frame_rate: ScreenShareFrameRate,
    /// Целевой битрейт кодировщика в битах в секунду.
    pub(crate) bitrate_bps: u32,
}

impl ScreenShareSelection {
    /// Создаёт параметры по умолчанию для системного окна выбора источника.
    pub(crate) fn browser_default() -> Self {
        Self {
            source_id: None,
            resolution: ScreenShareResolution::default(),
            frame_rate: ScreenShareFrameRate::default(),
            bitrate_bps: SCREEN_SHARE_BITRATE_LIMIT.max_bitrate_bps,
        }
    }

    /// Выбирает основной экран или первый доступный экран с качеством по умолчанию.
    pub(crate) fn default_for(sources: &[ScreenShareSource]) -> Option<Self> {
        let source = sources
            .iter()
            .find(|source| source.is_primary)
            .or_else(|| sources.first())?;

        Some(Self {
            source_id: Some(source.id.clone()),
            resolution: ScreenShareResolution::default(),
            frame_rate: ScreenShareFrameRate::default(),
            bitrate_bps: SCREEN_SHARE_BITRATE_LIMIT.max_bitrate_bps,
        })
    }

    /// Сохраняет допустимый выбор либо переносит качество на новый экран по умолчанию.
    pub(crate) fn normalized_for(
        current: Option<&Self>,
        sources: &[ScreenShareSource],
    ) -> Option<Self> {
        if let Some(current) = current
            && sources
                .iter()
                .any(|source| current.source_id.as_deref() == Some(source.id.as_str()))
        {
            return Some(current.clone());
        }

        let resolution =
            current.map_or_else(ScreenShareResolution::default, |value| value.resolution);
        let frame_rate =
            current.map_or_else(ScreenShareFrameRate::default, |value| value.frame_rate);
        Self::default_for(sources).map(|mut selection| {
            selection.resolution = resolution;
            selection.frame_rate = frame_rate;
            selection.bitrate_bps = current
                .map_or(SCREEN_SHARE_BITRATE_LIMIT.max_bitrate_bps, |value| {
                    value.bitrate_bps
                });
            selection
        })
    }
}

#[cfg(test)]
mod tests;

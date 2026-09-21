//! Чистая модель выбора источника демонстрации экрана.

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
}

impl ScreenShareResolution {
    /// Возвращает значение для HTML-списка.
    pub(crate) const fn value(self) -> &'static str {
        match self {
            Self::P720 => "720",
            Self::P1080 => "1080",
        }
    }

    /// Читает значение из HTML-списка.
    pub(crate) fn from_value(value: &str) -> Option<Self> {
        match value {
            "720" => Some(Self::P720),
            "1080" => Some(Self::P1080),
            _ => None,
        }
    }

    /// Возвращает название для итогового описания.
    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::P720 => "720p",
            Self::P1080 => "1080p",
        }
    }
}

/// Частота кадров демонстрации экрана.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ScreenShareFrameRate {
    /// До 15 кадров в секунду.
    Fps15,
    /// До 30 кадров в секунду.
    #[default]
    Fps30,
}

impl ScreenShareFrameRate {
    /// Возвращает значение для HTML-списка.
    pub(crate) const fn value(self) -> &'static str {
        match self {
            Self::Fps15 => "15",
            Self::Fps30 => "30",
        }
    }

    /// Читает значение из HTML-списка.
    pub(crate) fn from_value(value: &str) -> Option<Self> {
        match value {
            "15" => Some(Self::Fps15),
            "30" => Some(Self::Fps30),
            _ => None,
        }
    }

    /// Возвращает название для итогового описания.
    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::Fps15 => "15 кадров/с",
            Self::Fps30 => "30 кадров/с",
        }
    }
}

/// Объяснение автоматической коррекции качества.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScreenShareQualityAdjustment {
    /// Для 1080p частота снижена до 15 FPS.
    FrameRateReducedFor1080p,
    /// Для 30 FPS разрешение снижено до 720p.
    ResolutionReducedFor30Fps,
}

impl ScreenShareQualityAdjustment {
    /// Возвращает понятное пользователю объяснение.
    pub(crate) const fn message(self) -> &'static str {
        match self {
            Self::FrameRateReducedFor1080p => "Для 1080p частота кадров снижена до 15 FPS.",
            Self::ResolutionReducedFor30Fps => "Для 30 FPS разрешение снижено до 720p.",
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
    /// Идентификатор выбранного экрана.
    pub(crate) source_id: String,
    /// Выбранное разрешение.
    pub(crate) resolution: ScreenShareResolution,
    /// Выбранная частота кадров.
    pub(crate) frame_rate: ScreenShareFrameRate,
}

impl ScreenShareSelection {
    /// Выбирает основной экран или первый доступный экран с качеством по умолчанию.
    pub(crate) fn default_for(sources: &[ScreenShareSource]) -> Option<Self> {
        let source = sources
            .iter()
            .find(|source| source.is_primary)
            .or_else(|| sources.first())?;

        Some(Self {
            source_id: source.id.clone(),
            resolution: ScreenShareResolution::default(),
            frame_rate: ScreenShareFrameRate::default(),
        })
    }

    /// Сохраняет допустимый выбор либо переносит качество на новый экран по умолчанию.
    pub(crate) fn normalized_for(
        current: Option<&Self>,
        sources: &[ScreenShareSource],
    ) -> Option<Self> {
        if let Some(current) = current
            && sources.iter().any(|source| source.id == current.source_id)
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
            selection
        })
    }

    /// Выбирает разрешение и при необходимости корректирует FPS.
    pub(crate) fn select_resolution(
        &mut self,
        resolution: ScreenShareResolution,
    ) -> Option<ScreenShareQualityAdjustment> {
        self.resolution = resolution;
        if resolution == ScreenShareResolution::P1080
            && self.frame_rate == ScreenShareFrameRate::Fps30
        {
            self.frame_rate = ScreenShareFrameRate::Fps15;
            return Some(ScreenShareQualityAdjustment::FrameRateReducedFor1080p);
        }

        None
    }

    /// Выбирает FPS и при необходимости корректирует разрешение.
    pub(crate) fn select_frame_rate(
        &mut self,
        frame_rate: ScreenShareFrameRate,
    ) -> Option<ScreenShareQualityAdjustment> {
        self.frame_rate = frame_rate;
        if frame_rate == ScreenShareFrameRate::Fps30
            && self.resolution == ScreenShareResolution::P1080
        {
            self.resolution = ScreenShareResolution::P720;
            return Some(ScreenShareQualityAdjustment::ResolutionReducedFor30Fps);
        }

        None
    }

    /// Формирует доступное пользователю описание подтверждаемого выбора.
    pub(crate) fn confirmation_summary(&self, sources: &[ScreenShareSource]) -> Option<String> {
        let source = sources.iter().find(|source| source.id == self.source_id)?;

        Some(format!(
            "{} · {}×{} · {}, {} · Без звука",
            source.name,
            source.width,
            source.height,
            self.resolution.title(),
            self.frame_rate.title()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(id: &str, name: &str, is_primary: bool) -> ScreenShareSource {
        ScreenShareSource {
            id: id.to_owned(),
            name: name.to_owned(),
            width: 1_920,
            height: 1_080,
            preview_url: format!("data:image/png;base64,{id}"),
            is_primary,
        }
    }

    #[test]
    fn default_prefers_primary_source_even_when_it_is_not_first() {
        let sources = [
            source("secondary", "Экран 2", false),
            source("primary", "Основной экран", true),
        ];

        let selection = ScreenShareSelection::default_for(&sources).expect("есть экран");

        assert_eq!(selection.source_id, "primary");
        assert_eq!(selection.resolution, ScreenShareResolution::P720);
        assert_eq!(selection.frame_rate, ScreenShareFrameRate::Fps30);
    }

    #[test]
    fn default_uses_first_source_when_primary_is_absent() {
        let sources = [
            source("first", "Экран 1", false),
            source("second", "Экран 2", false),
        ];

        let selection = ScreenShareSelection::default_for(&sources).expect("есть экран");

        assert_eq!(selection.source_id, "first");
    }

    #[test]
    fn default_is_absent_without_sources() {
        assert_eq!(ScreenShareSelection::default_for(&[]), None);
    }

    #[test]
    fn normalization_preserves_valid_selection() {
        let sources = [source("first", "Экран 1", false)];
        let current = ScreenShareSelection {
            source_id: "first".to_owned(),
            resolution: ScreenShareResolution::P1080,
            frame_rate: ScreenShareFrameRate::Fps15,
        };

        assert_eq!(
            ScreenShareSelection::normalized_for(Some(&current), &sources),
            Some(current)
        );
    }

    #[test]
    fn normalization_moves_quality_to_new_default_source() {
        let sources = [source("primary", "Основной экран", true)];
        let current = ScreenShareSelection {
            source_id: "disconnected".to_owned(),
            resolution: ScreenShareResolution::P1080,
            frame_rate: ScreenShareFrameRate::Fps15,
        };

        let selection = ScreenShareSelection::normalized_for(Some(&current), &sources)
            .expect("есть новый экран");

        assert_eq!(selection.source_id, "primary");
        assert_eq!(selection.resolution, ScreenShareResolution::P1080);
        assert_eq!(selection.frame_rate, ScreenShareFrameRate::Fps15);
    }

    #[test]
    fn confirmation_names_source_resolution_quality_and_silent_capture() {
        let sources = [source("primary", "Основной экран", true)];
        let selection = ScreenShareSelection {
            source_id: "primary".to_owned(),
            resolution: ScreenShareResolution::P1080,
            frame_rate: ScreenShareFrameRate::Fps15,
        };

        assert_eq!(
            selection.confirmation_summary(&sources).as_deref(),
            Some("Основной экран · 1920×1080 · 1080p, 15 кадров/с · Без звука")
        );
    }

    #[test]
    fn confirmation_is_absent_for_a_stale_source() {
        let selection = ScreenShareSelection {
            source_id: "missing".to_owned(),
            resolution: ScreenShareResolution::P720,
            frame_rate: ScreenShareFrameRate::Fps30,
        };

        assert_eq!(selection.confirmation_summary(&[]), None);
    }

    #[test]
    fn selecting_1080p_reduces_frame_rate_and_explains_the_change() {
        let mut selection = ScreenShareSelection {
            source_id: "primary".to_owned(),
            resolution: ScreenShareResolution::P720,
            frame_rate: ScreenShareFrameRate::Fps30,
        };

        let adjustment = selection.select_resolution(ScreenShareResolution::P1080);

        assert_eq!(selection.resolution, ScreenShareResolution::P1080);
        assert_eq!(selection.frame_rate, ScreenShareFrameRate::Fps15);
        assert_eq!(
            adjustment,
            Some(ScreenShareQualityAdjustment::FrameRateReducedFor1080p)
        );
    }

    #[test]
    fn selecting_30_fps_reduces_resolution_and_explains_the_change() {
        let mut selection = ScreenShareSelection {
            source_id: "primary".to_owned(),
            resolution: ScreenShareResolution::P1080,
            frame_rate: ScreenShareFrameRate::Fps15,
        };

        let adjustment = selection.select_frame_rate(ScreenShareFrameRate::Fps30);

        assert_eq!(selection.resolution, ScreenShareResolution::P720);
        assert_eq!(selection.frame_rate, ScreenShareFrameRate::Fps30);
        assert_eq!(
            adjustment,
            Some(ScreenShareQualityAdjustment::ResolutionReducedFor30Fps)
        );
    }

    #[test]
    fn compatible_quality_change_does_not_show_an_adjustment() {
        let mut selection = ScreenShareSelection {
            source_id: "primary".to_owned(),
            resolution: ScreenShareResolution::P720,
            frame_rate: ScreenShareFrameRate::Fps30,
        };

        assert_eq!(
            selection.select_frame_rate(ScreenShareFrameRate::Fps15),
            None
        );
        assert_eq!(selection.resolution, ScreenShareResolution::P720);
        assert_eq!(selection.frame_rate, ScreenShareFrameRate::Fps15);
    }

    #[test]
    fn windows_tab_never_allows_confirmation() {
        let selection = ScreenShareSelection {
            source_id: "primary".to_owned(),
            resolution: ScreenShareResolution::P720,
            frame_rate: ScreenShareFrameRate::Fps30,
        };

        assert!(ScreenShareSourceTab::Screens.can_confirm(Some(&selection)));
        assert!(!ScreenShareSourceTab::Windows.can_confirm(Some(&selection)));
        assert!(!ScreenShareSourceTab::Screens.can_confirm(None));
    }
}

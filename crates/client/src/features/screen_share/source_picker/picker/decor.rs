//! Витрина качества демонстрации экрана.
//!
//! Панель позволяет выбрать произвольную комбинацию разрешения и частоты кадров,
//! включая 1440p, 4K и 60 FPS. Реальная модель выбора пока поддерживает только
//! 720p/1080p и 15/30 FPS, поэтому панель показывает выбранную комбинацию, а
//! запуск получает ближайшую поддерживаемую пару. Когда модель расширят,
//! изменится только сопоставление в этом модуле.

use super::super::model::{ScreenShareFrameRate, ScreenShareResolution};

/// Разрешение в панели качества.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum MockResolution {
    /// 1280×720.
    #[default]
    P720,
    /// 1920×1080.
    P1080,
    /// 2560×1440.
    P1440,
    /// 3840×2160.
    Uhd4K,
}

impl MockResolution {
    /// Все варианты в порядке возрастания.
    pub(super) const ALL: [Self; 4] = [Self::P720, Self::P1080, Self::P1440, Self::Uhd4K];

    /// Возвращает значение для HTML-списка.
    pub(super) const fn value(self) -> &'static str {
        match self {
            Self::P720 => "720",
            Self::P1080 => "1080",
            Self::P1440 => "1440",
            Self::Uhd4K => "2160",
        }
    }

    /// Читает значение из HTML-списка.
    pub(super) fn from_value(value: &str) -> Option<Self> {
        match value {
            "720" => Some(Self::P720),
            "1080" => Some(Self::P1080),
            "1440" => Some(Self::P1440),
            "2160" => Some(Self::Uhd4K),
            _ => None,
        }
    }

    /// Возвращает подпись варианта.
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::P720 => "720p",
            Self::P1080 => "1080p",
            Self::P1440 => "1440p",
            Self::Uhd4K => "4K",
        }
    }

    /// Возвращает уточнение под подписью.
    pub(super) const fn hint(self) -> &'static str {
        match self {
            Self::P720 => "HD",
            Self::P1080 => "Full HD",
            Self::P1440 => "2K",
            Self::Uhd4K => "Ultra HD",
        }
    }

    /// Возвращает поддерживаемое моделью разрешение для запуска.
    ///
    /// 1440p и 4K пока не выражаются моделью выбора, поэтому запуск получает
    /// максимальное доступное разрешение — 1080p.
    pub(super) const fn supported(self) -> ScreenShareResolution {
        match self {
            Self::P720 => ScreenShareResolution::P720,
            Self::P1080 | Self::P1440 | Self::Uhd4K => ScreenShareResolution::P1080,
        }
    }
}

/// Частота кадров в панели качества.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum MockFrameRate {
    /// 15 кадров в секунду.
    Fps15,
    /// 24 кадра в секунду.
    Fps24,
    /// 30 кадров в секунду.
    #[default]
    Fps30,
    /// 60 кадров в секунду.
    Fps60,
}

impl MockFrameRate {
    /// Все варианты в порядке возрастания.
    pub(super) const ALL: [Self; 4] = [Self::Fps15, Self::Fps24, Self::Fps30, Self::Fps60];

    /// Возвращает значение для HTML-списка.
    pub(super) const fn value(self) -> &'static str {
        match self {
            Self::Fps15 => "15",
            Self::Fps24 => "24",
            Self::Fps30 => "30",
            Self::Fps60 => "60",
        }
    }

    /// Читает значение из HTML-списка.
    pub(super) fn from_value(value: &str) -> Option<Self> {
        match value {
            "15" => Some(Self::Fps15),
            "24" => Some(Self::Fps24),
            "30" => Some(Self::Fps30),
            "60" => Some(Self::Fps60),
            _ => None,
        }
    }

    /// Возвращает подпись варианта.
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Fps15 => "15 FPS",
            Self::Fps24 => "24 FPS",
            Self::Fps30 => "30 FPS",
            Self::Fps60 => "60 FPS",
        }
    }

    /// Возвращает уточнение под подписью.
    pub(super) const fn hint(self) -> &'static str {
        match self {
            Self::Fps15 => "Экономно",
            Self::Fps24 => "Кино",
            Self::Fps30 => "Плавно",
            Self::Fps60 => "Максимум",
        }
    }

    /// Возвращает поддерживаемую моделью частоту для выбранного разрешения.
    ///
    /// 1080p и выше ограничены 15 кадрами в секунду, а 720p — 30 кадрами.
    pub(super) const fn supported_for(self, resolution: MockResolution) -> ScreenShareFrameRate {
        match (resolution, self) {
            (MockResolution::P720, Self::Fps15) => ScreenShareFrameRate::Fps15,
            (MockResolution::P720, Self::Fps24 | Self::Fps30 | Self::Fps60) => {
                ScreenShareFrameRate::Fps30
            }
            (_, _) => ScreenShareFrameRate::Fps15,
        }
    }
}

/// Возвращает поддерживаемую моделью пару для выбранной комбинации.
pub(super) fn supported_pair(
    resolution: MockResolution,
    frame_rate: MockFrameRate,
) -> (ScreenShareResolution, ScreenShareFrameRate) {
    (resolution.supported(), frame_rate.supported_for(resolution))
}

/// Проверяет, что комбинация запускается без упрощения.
pub(super) fn is_supported(resolution: MockResolution, frame_rate: MockFrameRate) -> bool {
    matches!(
        (resolution, frame_rate),
        (
            MockResolution::P720,
            MockFrameRate::Fps15 | MockFrameRate::Fps30
        ) | (MockResolution::P1080, MockFrameRate::Fps15)
    )
}

/// Уровень качества в панели пресетов.
#[derive(Debug, Clone, Copy)]
pub(super) struct QualityPreset {
    /// Название уровня.
    pub(super) name: &'static str,
    /// Крупная подпись с основным параметром.
    pub(super) headline: &'static str,
    /// Строка с частотой кадров и битрейтом.
    pub(super) meta: &'static str,
    /// Параметры пресета, если его можно применить.
    pub(super) apply: Option<(MockResolution, MockFrameRate)>,
    /// Признак тарифного ограничения.
    pub(super) requires_subscription: bool,
}

/// Пресеты, которые показывает панель качества.
///
/// Первые два уровня применяют реальные параметры демонстрации. Третий уровень
/// оставлен витриной тарифа: у него нет параметров, поэтому он не влияет на
/// подтверждаемый выбор.
pub(super) static QUALITY_PRESETS: [QualityPreset; 3] = [
    QualityPreset {
        name: "Стандартное",
        headline: "720p",
        meta: "30 к/с · 1 Мбит/с",
        apply: Some((MockResolution::P720, MockFrameRate::Fps30)),
        requires_subscription: false,
    },
    QualityPreset {
        name: "Улучшенное",
        headline: "1080p",
        meta: "15 к/с · 3 Мбит/с",
        apply: Some((MockResolution::P1080, MockFrameRate::Fps15)),
        requires_subscription: false,
    },
    QualityPreset {
        name: "Высокое",
        headline: "До 4K",
        meta: "60 к/с · 5 Мбит/с",
        apply: None,
        requires_subscription: true,
    },
];

/// Минимальный битрейт ползунка в Мбит/с.
pub(super) const BITRATE_MIN: f64 = 0.25;

/// Максимальный битрейт бесплатного доступа в Мбит/с.
pub(super) const BITRATE_MAX_FREE: f64 = 1.0;

/// Максимальный битрейт расширенного тарифа в Мбит/с.
///
/// Шкала ползунка всегда доходит до этого значения, даже когда доступ пользователя
/// ограничен бесплатным лимитом.
pub(super) const BITRATE_MAX_SUBSCRIBED: f64 = 5.0;

/// Подписи засечек под ползунком битрейта.
pub(super) const BITRATE_TICKS: [f64; 6] = [0.25, 1.0, 2.0, 3.0, 4.0, 5.0];

/// Новые параметры качества, выбранные в панели.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct QualityChange {
    /// Разрешение, которое видит пользователь.
    pub(super) mock_resolution: MockResolution,
    /// Частота кадров, которую видит пользователь.
    pub(super) mock_frame_rate: MockFrameRate,
    /// Разрешение, которое получит запуск.
    pub(super) resolution: ScreenShareResolution,
    /// Частота кадров, которую получит запуск.
    pub(super) frame_rate: ScreenShareFrameRate,
}

/// Формирует изменение качества с ближайшей поддерживаемой парой.
pub(super) fn change_for(
    mock_resolution: MockResolution,
    mock_frame_rate: MockFrameRate,
) -> QualityChange {
    let (resolution, frame_rate) = supported_pair(mock_resolution, mock_frame_rate);
    QualityChange {
        mock_resolution,
        mock_frame_rate,
        resolution,
        frame_rate,
    }
}

/// Подбирает максимальное поддерживаемое качество с запасом по частоте кадров.
pub(super) fn optimize() -> QualityChange {
    change_for(MockResolution::P1080, MockFrameRate::Fps15)
}

/// Возвращает индекс пресета, совпадающего с текущей комбинацией.
///
/// `None` означает, что пользователь собрал собственные настройки и ни одна
/// карточка пресета не должна выглядеть выбранной.
pub(super) fn preset_index(resolution: MockResolution, frame_rate: MockFrameRate) -> Option<usize> {
    QUALITY_PRESETS
        .iter()
        .position(|preset| preset.apply == Some((resolution, frame_rate)))
}

/// Тон итоговой подсказки о качестве.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AssessmentTone {
    /// Сбалансированные настройки.
    Good,
    /// Настройки подходят для части сценариев.
    Info,
}

/// Итоговая оценка выбранных параметров.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct QualityAssessment {
    /// Визуальный тон подсказки.
    pub(super) tone: AssessmentTone,
    /// Заполнение индикатора от 1 до 4.
    pub(super) score: u8,
    /// Заголовок подсказки.
    pub(super) title: &'static str,
    /// Пояснение к настройкам.
    pub(super) text: &'static str,
}

/// Оценивает выбранную комбинацию разрешения и частоты кадров.
///
/// Комбинации выше текущих возможностей модели получают информативный тон и
/// честно сообщают, какое качество будет использовано при запуске.
pub(super) fn assess(resolution: MockResolution, frame_rate: MockFrameRate) -> QualityAssessment {
    match (resolution, frame_rate) {
        (MockResolution::P1080, MockFrameRate::Fps15) => QualityAssessment {
            tone: AssessmentTone::Good,
            score: 4,
            title: "Чёткая картинка для текста",
            text: "1080p с 15 кадрами в секунду хорошо передаёт документы, интерфейсы и мелкие детали.",
        },
        (MockResolution::P720, MockFrameRate::Fps30) => QualityAssessment {
            tone: AssessmentTone::Good,
            score: 3,
            title: "Плавная картинка для движения",
            text: "720p с 30 кадрами в секунду подходит для видео, игр и динамичных сцен.",
        },
        (MockResolution::P720, MockFrameRate::Fps15) => QualityAssessment {
            tone: AssessmentTone::Good,
            score: 2,
            title: "Экономный режим",
            text: "Частота кадров снижена: движение будет менее плавным, зато нагрузка на сеть ниже.",
        },
        (MockResolution::P720, MockFrameRate::Fps24 | MockFrameRate::Fps60) => QualityAssessment {
            tone: AssessmentTone::Info,
            score: 3,
            title: "720p работает до 30 кадров в секунду",
            text: "Сейчас трансляция идёт в 720p с 30 кадрами в секунду.",
        },
        (MockResolution::P1080, _) => QualityAssessment {
            tone: AssessmentTone::Info,
            score: 3,
            title: "1080p работает при 15 кадрах в секунду",
            text: "Сейчас трансляция идёт в 1080p с 15 кадрами в секунду.",
        },
        (MockResolution::P1440 | MockResolution::Uhd4K, _) => QualityAssessment {
            tone: AssessmentTone::Info,
            score: 2,
            title: "Высокое разрешение появится позже",
            text: "Сейчас трансляция идёт в 1080p с 15 кадрами в секунду.",
        },
    }
}

/// Форматирует битрейт без лишних нулей после запятой.
///
/// Значения вроде `1.0` печатаются как `1`, а дробные — как `1,5` или `0,25`.
pub(super) fn format_bitrate(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    let text = if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        let raw = format!("{rounded:.2}");
        raw.trim_end_matches('0').trim_end_matches('.').to_owned()
    };

    text.replace('.', ",")
}

/// Переводит битрейт в долю полной шкалы от `0` до `1`.
///
/// Шкала всегда покрывает диапазон от [`BITRATE_MIN`] до
/// [`BITRATE_MAX_SUBSCRIBED`], поэтому закрытая тарифом зона видна целиком.
pub(super) fn bitrate_fraction(value: f64) -> f64 {
    ((value - BITRATE_MIN) / (BITRATE_MAX_SUBSCRIBED - BITRATE_MIN)).clamp(0.0, 1.0)
}

/// Возвращает строковое значение булева ARIA-состояния.
///
/// Dioxus принимает ARIA-атрибуты только строками, поэтому состояния
/// переключателей и вкладок переводятся в `"true"` и `"false"`.
pub(super) fn aria_flag(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

#[cfg(test)]
mod tests;

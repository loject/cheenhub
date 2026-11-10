//! Витрина качества демонстрации экрана.
//!
//! Подписи и варианты выбора разрешения, частоты кадров и битрейта.

use super::super::model::{ScreenShareFrameRate, ScreenShareResolution};

impl ScreenShareResolution {
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
}

impl ScreenShareFrameRate {
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
}

/// Возвращает поддерживаемую моделью пару для выбранной комбинации.
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
    pub(super) apply: Option<(ScreenShareResolution, ScreenShareFrameRate)>,
    /// Признак тарифного ограничения.
    pub(super) requires_subscription: bool,
}

/// Пресеты, которые показывает панель качества.
///
/// Карточки меняют разрешение и частоту кадров, сохраняя выбранный битрейт.
pub(super) static QUALITY_PRESETS: [QualityPreset; 3] = [
    QualityPreset {
        name: "Стандартное",
        headline: "720p",
        meta: "30 к/с",
        apply: Some((ScreenShareResolution::P720, ScreenShareFrameRate::Fps30)),
        requires_subscription: false,
    },
    QualityPreset {
        name: "Улучшенное",
        headline: "1080p",
        meta: "15 к/с",
        apply: Some((ScreenShareResolution::P1080, ScreenShareFrameRate::Fps15)),
        requires_subscription: false,
    },
    QualityPreset {
        name: "Высокое",
        headline: "До 4K",
        meta: "60 к/с",
        apply: Some((ScreenShareResolution::Uhd4K, ScreenShareFrameRate::Fps60)),
        requires_subscription: false,
    },
];

/// Минимальный битрейт ползунка в Мбит/с.
pub(super) const BITRATE_MIN: f64 = 0.25;

/// Максимальный битрейт бесплатного доступа в Мбит/с.
pub(super) const BITRATE_MAX_FREE: f64 =
    cheenhub_contracts::video_presets::SCREEN_SHARE_BITRATE_LIMIT.max_bitrate_bps as f64
        / 1_000_000.0;

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
    /// Разрешение, которое получит запуск.
    pub(super) resolution: ScreenShareResolution,
    /// Частота кадров, которую получит запуск.
    pub(super) frame_rate: ScreenShareFrameRate,
}

/// Формирует изменение качества без преобразования параметров.
pub(super) fn change_for(
    resolution: ScreenShareResolution,
    frame_rate: ScreenShareFrameRate,
) -> QualityChange {
    QualityChange {
        resolution,
        frame_rate,
    }
}

/// Подбирает максимальное поддерживаемое качество с запасом по частоте кадров.
pub(super) fn optimize() -> QualityChange {
    change_for(ScreenShareResolution::P1080, ScreenShareFrameRate::Fps15)
}

/// Возвращает индекс пресета, совпадающего с текущей комбинацией.
///
/// `None` означает, что пользователь собрал собственные настройки и ни одна
/// карточка пресета не должна выглядеть выбранной.
pub(super) fn preset_index(
    resolution: ScreenShareResolution,
    frame_rate: ScreenShareFrameRate,
) -> Option<usize> {
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
/// Оценка описывает возможные компромиссы выбранной комбинации.
pub(super) fn assess(
    resolution: ScreenShareResolution,
    frame_rate: ScreenShareFrameRate,
) -> QualityAssessment {
    match (resolution, frame_rate) {
        (ScreenShareResolution::P1080, ScreenShareFrameRate::Fps15) => QualityAssessment {
            tone: AssessmentTone::Good,
            score: 4,
            title: "Чёткая картинка для текста",
            text: "1080p с 15 кадрами в секунду хорошо передаёт документы, интерфейсы и мелкие детали.",
        },
        (ScreenShareResolution::P720, ScreenShareFrameRate::Fps30) => QualityAssessment {
            tone: AssessmentTone::Good,
            score: 3,
            title: "Плавная картинка для движения",
            text: "720p с 30 кадрами в секунду подходит для видео, игр и динамичных сцен.",
        },
        (ScreenShareResolution::P720, ScreenShareFrameRate::Fps15) => QualityAssessment {
            tone: AssessmentTone::Good,
            score: 2,
            title: "Экономный режим",
            text: "Частота кадров снижена: движение будет менее плавным, зато нагрузка на сеть ниже.",
        },
        (
            ScreenShareResolution::P720,
            ScreenShareFrameRate::Fps24 | ScreenShareFrameRate::Fps60,
        ) => QualityAssessment {
            tone: AssessmentTone::Info,
            score: 3,
            title: "Выбранная частота кадров",
            text: "Плавность зависит от источника и возможностей устройства.",
        },
        (ScreenShareResolution::P1080, _) => QualityAssessment {
            tone: AssessmentTone::Info,
            score: 3,
            title: "Высокая детализация",
            text: "Качество изображения зависит от доступного битрейта и устройства.",
        },
        (ScreenShareResolution::P1440 | ScreenShareResolution::Uhd4K, _) => QualityAssessment {
            tone: AssessmentTone::Info,
            score: 2,
            title: "Высокое разрешение",
            text: "Для мелких деталей может потребоваться более высокий битрейт.",
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

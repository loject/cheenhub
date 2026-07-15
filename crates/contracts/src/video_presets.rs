//! Общие контракты пресетов исходящего видео.

use serde::{Deserialize, Serialize};

/// Источник исходящего видеопотока.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoStreamSource {
    /// Камера пользователя.
    Camera,
    /// Демонстрация экрана пользователя.
    ScreenShare,
}

/// Стабильный идентификатор пресета исходящего видео.
///
/// Идентификатор отделён от параметров, чтобы тариф или подписка могли выдавать
/// пользователю дополнительный набор разрешённых пресетов.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoPresetId {
    /// Камера 1280x720 с частотой до 24 кадров в секунду.
    Camera720p24,
    /// Демонстрация экрана 1280x720 с частотой до 30 кадров в секунду.
    Screen720p30,
    /// Демонстрация экрана 1920x1080 с частотой до 15 кадров в секунду.
    Screen1080p15,
}

/// Числовые параметры пресета исходящего видео.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoPresetSpec {
    /// Источник видеопотока.
    pub source: VideoStreamSource,
    /// Ширина закодированного кадра.
    pub width: u32,
    /// Высота закодированного кадра.
    pub height: u32,
    /// Максимальная частота кадров в секунду.
    pub max_fps: u32,
    /// Целевой bitrate кодировщика в битах в секунду.
    pub bitrate_bps: u32,
}

impl VideoPresetId {
    /// Возвращает числовые параметры пресета.
    pub const fn spec(self) -> VideoPresetSpec {
        match self {
            Self::Camera720p24 => VideoPresetSpec {
                source: VideoStreamSource::Camera,
                width: 1280,
                height: 720,
                max_fps: 24,
                bitrate_bps: 700_000,
            },
            Self::Screen720p30 => VideoPresetSpec {
                source: VideoStreamSource::ScreenShare,
                width: 1280,
                height: 720,
                max_fps: 30,
                bitrate_bps: 1_500_000,
            },
            Self::Screen1080p15 => VideoPresetSpec {
                source: VideoStreamSource::ScreenShare,
                width: 1920,
                height: 1080,
                max_fps: 15,
                bitrate_bps: 1_500_000,
            },
        }
    }
}

/// Базовые пресеты камеры без расширений подписки.
pub const BASE_CAMERA_VIDEO_PRESETS: &[VideoPresetId] = &[VideoPresetId::Camera720p24];

/// Базовые пресеты демонстрации экрана без расширений подписки.
pub const BASE_SCREEN_SHARE_VIDEO_PRESETS: &[VideoPresetId] =
    &[VideoPresetId::Screen720p30, VideoPresetId::Screen1080p15];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_presets_belong_to_expected_sources() {
        assert!(
            BASE_CAMERA_VIDEO_PRESETS
                .iter()
                .all(|preset| preset.spec().source == VideoStreamSource::Camera)
        );
        assert!(
            BASE_SCREEN_SHARE_VIDEO_PRESETS
                .iter()
                .all(|preset| preset.spec().source == VideoStreamSource::ScreenShare)
        );
    }

    #[test]
    fn preset_ids_round_trip_through_json() {
        let encoded =
            serde_json::to_string(&VideoPresetId::Screen1080p15).expect("preset id serializes");
        let decoded: VideoPresetId =
            serde_json::from_str(&encoded).expect("preset id deserializes");

        assert_eq!(decoded, VideoPresetId::Screen1080p15);
    }
}

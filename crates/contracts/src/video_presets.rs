//! Общие ограничения битрейта и параметры кодирования исходящего видео.

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

/// Тарифный предел видеопотока, независимый от размеров и частоты кадров.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoBitrateLimit {
    /// Источник, к которому применяется ограничение.
    pub source: VideoStreamSource,
    /// Максимальный средний битрейт в битах в секунду.
    pub max_bitrate_bps: u32,
}

/// Бесплатный предел исходящего видео камеры.
pub const CAMERA_BITRATE_LIMIT: VideoBitrateLimit = VideoBitrateLimit {
    source: VideoStreamSource::Camera,
    max_bitrate_bps: 700_000,
};

/// Бесплатный предел исходящей демонстрации экрана.
pub const SCREEN_SHARE_BITRATE_LIMIT: VideoBitrateLimit = VideoBitrateLimit {
    source: VideoStreamSource::ScreenShare,
    max_bitrate_bps: 1_500_000,
};

/// Настройки кодирования исходящего видео.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoEncodingSettings {
    /// Источник видеопотока.
    pub source: VideoStreamSource,
    /// Ширина кадра камеры или максимальная ширина кадра демонстрации экрана.
    pub width: u32,
    /// Высота кадра камеры или максимальная высота кадра демонстрации экрана.
    pub height: u32,
    /// Максимальная частота кадров в секунду.
    pub max_fps: u32,
    /// Целевой bitrate кодировщика в битах в секунду.
    pub bitrate_bps: u32,
}

#[cfg(test)]
mod tests;

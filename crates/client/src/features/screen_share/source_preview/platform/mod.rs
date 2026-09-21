//! Платформенный контракт статических превью источников демонстрации экрана.

use std::fmt::{Display, Formatter};

mod native;

pub use native::{load_previews, selection_available};

/// Статическое превью физического монитора.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonitorPreview {
    /// Идентификатор монитора, стабильный в пределах одной загрузки списка.
    pub id: String,
    /// Отображаемое имя монитора.
    pub display_name: String,
    /// Ширина монитора в физических пикселях.
    pub width: u32,
    /// Высота монитора в физических пикселях.
    pub height: u32,
    /// Признак основного монитора Windows.
    pub primary: bool,
    /// PNG-изображение превью.
    pub png_bytes: Vec<u8>,
}

/// Ошибка загрузки статических превью мониторов.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewLoadError {
    message: String,
}

impl Display for PreviewLoadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PreviewLoadError {}

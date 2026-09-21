//! Получение статических превью источников демонстрации экрана.

mod platform;

pub(crate) use platform::{MonitorPreview, load_previews, selection_available};

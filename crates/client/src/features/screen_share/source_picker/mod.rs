//! Выбор экрана и качества перед началом демонстрации.

mod model;
mod picker;

pub(crate) use model::{ScreenShareSelection, ScreenShareSource, ScreenShareSourcePickerState};

#[cfg(test)]
pub(crate) use model::{ScreenShareFrameRate, ScreenShareResolution};
pub(crate) use picker::ScreenShareSourcePicker;

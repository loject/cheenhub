//! Выбор экрана и качества перед началом демонстрации.

mod model;
mod picker;

pub(crate) use model::{ScreenShareSelection, ScreenShareSource, ScreenShareSourcePickerState};
pub(crate) use picker::ScreenShareSourcePicker;

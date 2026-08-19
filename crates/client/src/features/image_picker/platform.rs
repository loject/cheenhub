//! Выбор реализации image picker для текущей платформы.

#[cfg(target_os = "android")]
#[path = "platform/android.rs"]
mod implementation;

#[cfg(not(target_os = "android"))]
#[path = "platform/file_input.rs"]
mod implementation;

pub(crate) use implementation::ImagePickerButton;

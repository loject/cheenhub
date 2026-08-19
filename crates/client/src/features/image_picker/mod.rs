//! Платформенный выбор изображения для вложения в сообщение.

mod backend;
mod platform;

pub(crate) use backend::{ImagePickerOutcome, PickedImage};
pub(crate) use platform::ImagePickerButton;

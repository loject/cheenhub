//! Выбор реализации статических превью для текущей native-платформы.

#[cfg(all(target_os = "windows", feature = "windows"))]
#[path = "windows.rs"]
mod implementation;
#[cfg(not(all(target_os = "windows", feature = "windows")))]
#[path = "unsupported.rs"]
mod implementation;

pub use implementation::{load_previews, selection_available};

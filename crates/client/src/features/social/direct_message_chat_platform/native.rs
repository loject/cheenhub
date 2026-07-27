//! Выбор политики начальной видимости чата для native-платформы.

#[cfg(target_os = "android")]
#[path = "android.rs"]
mod platform;
#[cfg(not(target_os = "android"))]
#[path = "default.rs"]
mod platform;

pub(crate) use platform::open_by_default;

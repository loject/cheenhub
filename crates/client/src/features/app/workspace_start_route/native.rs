//! Выбор реализации стартового маршрута для native-платформы.

#[cfg(target_os = "android")]
#[path = "android.rs"]
mod platform;
#[cfg(not(target_os = "android"))]
#[path = "default.rs"]
mod platform;

pub(crate) use platform::resolve;

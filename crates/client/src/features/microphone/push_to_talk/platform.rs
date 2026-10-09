//! Выбирает реализацию глобального ввода, не раскрывая platform API соседним модулям.

#[cfg(not(any(
    feature = "web",
    all(feature = "desktop", feature = "windows", target_os = "windows")
)))]
mod unsupported;
#[cfg(feature = "web")]
mod web;
#[cfg(all(
    feature = "desktop",
    feature = "windows",
    target_os = "windows",
    not(feature = "web")
))]
mod windows;

#[cfg(not(any(
    feature = "web",
    all(feature = "desktop", feature = "windows", target_os = "windows")
)))]
pub(in crate::features::microphone) use unsupported::{
    Monitor, key_label, record_binding, supported, unsupported_reason,
};
#[cfg(feature = "web")]
pub(in crate::features::microphone) use web::{
    key_label, record_binding, supported, unsupported_reason,
};
#[cfg(all(
    feature = "desktop",
    feature = "windows",
    target_os = "windows",
    not(feature = "web")
))]
pub(in crate::features::microphone) use windows::{
    Monitor, key_label, record_binding, supported, unsupported_reason,
};

//! Выбор платформенного интервала обновления аудиоустройств.

#[cfg(all(target_os = "linux", feature = "linux"))]
#[path = "linux.rs"]
mod implementation;
#[cfg(not(all(target_os = "linux", feature = "linux")))]
#[path = "unsupported.rs"]
mod implementation;

pub(super) use implementation::Timer;

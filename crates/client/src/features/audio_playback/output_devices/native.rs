//! Выбор платформенной реализации перечисления устройств вывода аудио.

#[cfg(target_os = "android")]
#[path = "native/android.rs"]
mod implementation;
#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    any(feature = "windows", feature = "linux", feature = "macos")
))]
#[path = "native/desktop.rs"]
mod implementation;
#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    not(any(feature = "windows", feature = "linux", feature = "macos"))
))]
#[path = "native/unsupported.rs"]
mod implementation;
#[cfg(target_arch = "wasm32")]
#[path = "web.rs"]
mod implementation;

pub(crate) use implementation::enumerate_audio_output_devices;

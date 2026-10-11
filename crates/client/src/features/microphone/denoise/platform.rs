//! Платформенные значения настроек подавления шума.

/// Выбирает RNNoise для desktop Windows/Linux и браузерный режим для веба, Off для остальных платформ.
pub(super) fn default_mode() -> super::DenoiseMode {
    if super::available()
        && cfg!(any(
            all(feature = "windows", target_os = "windows"),
            all(feature = "linux", target_os = "linux")
        ))
    {
        super::DenoiseMode::Rnnoise
    } else if browser_available() {
        super::DenoiseMode::Browser
    } else {
        super::DenoiseMode::Off
    }
}

#[cfg(test)]
mod tests;

/// Указывает, содержит ли сборка браузерный backend захвата.
pub(super) fn browser_available() -> bool {
    cfg!(feature = "web")
}

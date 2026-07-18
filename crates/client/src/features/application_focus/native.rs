//! Выбор платформенной реализации проверки фокуса приложения.

#[cfg(target_arch = "wasm32")]
pub(crate) use super::web::application_is_focused;

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    feature = "desktop"
))]
/// Возвращает, видимо ли desktop-окно и принадлежит ли ему фокус.
pub(crate) fn application_is_focused() -> bool {
    let window = dioxus::desktop::window();
    window.is_visible() && window.is_focused()
}

#[cfg(any(
    all(not(target_arch = "wasm32"), target_os = "android"),
    all(
        not(target_arch = "wasm32"),
        not(target_os = "android"),
        not(feature = "desktop")
    )
))]
pub(crate) use super::unsupported::application_is_focused;

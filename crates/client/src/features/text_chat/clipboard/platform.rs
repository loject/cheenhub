//! Выбор реализации буфера обмена для текущей клиентской платформы.

#[cfg(target_arch = "wasm32")]
#[path = "platform/web.rs"]
mod implementation;
#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    feature = "desktop"
))]
#[path = "platform/desktop.rs"]
mod implementation;
#[cfg(any(
    target_os = "android",
    all(not(target_arch = "wasm32"), not(feature = "desktop"))
))]
#[path = "platform/unsupported.rs"]
mod implementation;
pub(crate) use implementation::read_pasted_image;

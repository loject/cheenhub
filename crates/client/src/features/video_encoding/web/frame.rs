//! Отображаемые размеры и время жизни входного браузерного `VideoFrame`.

use std::cell::Cell;

use dioxus::prelude::warn;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[cfg(test)]
mod tests;

/// Входной браузерный кадр, закрываемый явно или при освобождении wrapper'а.
pub(crate) struct BrowserVideoFrame {
    /// Исходный кадр для передачи браузерному кодировщику без промежуточного копирования.
    pub(super) frame: VideoFrame,
    closed: Cell<bool>,
}

impl BrowserVideoFrame {
    /// Создаёт wrapper из кадра, полученного от браузерного источника.
    pub(crate) fn from_js_value(value: &JsValue) -> Self {
        Self {
            frame: value.unchecked_ref::<VideoFrame>().clone(),
            closed: Cell::new(false),
        }
    }

    /// Закрывает кадр и освобождает браузерный ресурс.
    pub(crate) fn close(&self) {
        if let Err(error) = close_once(&self.closed, || self.frame.close()) {
            warn!(error = ?error, "failed to close browser VideoFrame");
        }
    }

    /// Закрывает полученный от reader кадр, если значение действительно является VideoFrame.
    pub(crate) fn close_js_value(value: &JsValue) -> bool {
        let Some(frame) = value.dyn_ref::<VideoFrame>() else {
            return false;
        };
        Self {
            frame: frame.clone(),
            closed: Cell::new(false),
        }
        .close();
        true
    }

    /// Возвращает временную метку входного кадра в микросекундах.
    pub(crate) fn timestamp_us(&self) -> u64 {
        self.frame.timestamp().max(0.0).round() as u64
    }

    /// Возвращает реальные отображаемые размеры кадра, независимо от стартовых настроек track.
    pub(crate) fn display_dimensions(&self) -> (u32, u32) {
        (self.frame.display_width(), self.frame.display_height())
    }
}

fn close_once<E>(closed: &Cell<bool>, close: impl FnOnce() -> Result<(), E>) -> Result<bool, E> {
    if closed.replace(true) {
        return Ok(false);
    }
    close()?;
    Ok(true)
}

impl Drop for BrowserVideoFrame {
    fn drop(&mut self) {
        self.close();
    }
}

#[wasm_bindgen]
extern "C" {
    /// Браузерный объект кадра, передаваемый WebCodecs без изменения его пропорций.
    #[wasm_bindgen(js_name = VideoFrame)]
    #[derive(Clone)]
    pub(super) type VideoFrame;

    #[wasm_bindgen(method, catch, js_name = close)]
    fn close(this: &VideoFrame) -> Result<(), JsValue>;

    #[wasm_bindgen(method, getter)]
    fn timestamp(this: &VideoFrame) -> f64;

    #[wasm_bindgen(method, getter, js_name = displayWidth)]
    fn display_width(this: &VideoFrame) -> u32;

    #[wasm_bindgen(method, getter, js_name = displayHeight)]
    fn display_height(this: &VideoFrame) -> u32;
}

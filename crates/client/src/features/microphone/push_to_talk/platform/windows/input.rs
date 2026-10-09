//! Декодирование Windows событий кнопок и системных названий без UI зависимостей.

use super::super::super::key::PushToTalkKey;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyNameTextW, MAPVK_VK_TO_VSC_EX, MapVirtualKeyW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Нажатие или отпускание одной кнопки, пригодной для удержания.
pub(super) struct InputEvent {
    /// Проверенный код кнопки.
    pub(super) key: PushToTalkKey,
    /// `true` при нажатии, `false` при отпускании.
    pub(super) pressed: bool,
}

/// Преобразует только клавиатурные переходы, различая стороны модификаторов.
pub(super) fn keyboard(kind: u32, event: &KBDLLHOOKSTRUCT) -> Option<InputEvent> {
    if !matches!(kind, WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP) {
        return None;
    }
    let code = super::key_code(event.vkCode, event.scanCode, event.flags);
    Some(InputEvent {
        key: PushToTalkKey::from_code(u16::try_from(code).ok()?)?,
        pressed: matches!(kind, WM_KEYDOWN | WM_SYSKEYDOWN),
    })
}

/// Преобразует пять кнопок мыши; движение и колесо не являются удерживаемой кнопкой.
pub(super) fn mouse(kind: u32, mouse_data: u32) -> Option<InputEvent> {
    let (code, pressed) = match kind {
        WM_LBUTTONDOWN => (1, true),
        WM_LBUTTONUP => (1, false),
        WM_RBUTTONDOWN => (2, true),
        WM_RBUTTONUP => (2, false),
        WM_MBUTTONDOWN => (4, true),
        WM_MBUTTONUP => (4, false),
        WM_XBUTTONDOWN | WM_XBUTTONUP => {
            let code = match mouse_data >> 16 {
                1 => 5,
                2 => 6,
                _ => return None,
            };
            (code, kind == WM_XBUTTONDOWN)
        }
        _ => return None,
    };
    Some(InputEvent {
        key: PushToTalkKey::from_code(code)?,
        pressed,
    })
}

/// Даёт системное название клавиши и привычные Mouse1–Mouse5 для мыши.
pub(super) fn key_label(key: PushToTalkKey) -> String {
    let code = key.code();
    if key.is_mouse() || matches!(code, 8 | 9 | 13 | 27 | 32 | 0xa0..=0xa5) {
        return key.fallback_label();
    }
    let scan = unsafe { MapVirtualKeyW(u32::from(code), MAPVK_VK_TO_VSC_EX) };
    let parameter = ((scan & 0xff) << 16) | if scan & 0xff00 != 0 { 1 << 24 } else { 0 };
    let mut name = [0_u16; 128];
    let length = unsafe { GetKeyNameTextW(parameter as i32, name.as_mut_ptr(), name.len() as i32) };
    if length > 0 {
        String::from_utf16_lossy(&name[..length as usize])
    } else {
        key.fallback_label()
    }
}

#[cfg(test)]
mod tests;

//! Поиск физического монитора по идентификатору Windows.

use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};
use windows::core::BOOL;

/// Находит выбранный физический монитор перед запуском захвата.
pub(super) fn find_monitor(source_id: &str) -> Result<HMONITOR, String> {
    unsafe extern "system" fn callback(
        handle: HMONITOR,
        _device_context: HDC,
        _rect: *mut RECT,
        state: LPARAM,
    ) -> BOOL {
        let state = unsafe { &mut *(state.0 as *mut MonitorSearch) };
        if state.handle.is_none()
            && let Some(id) = monitor_id(handle)
            && id == state.source_id
        {
            state.handle = Some(handle);
        }
        true.into()
    }

    let mut search = MonitorSearch {
        source_id,
        handle: None,
    };
    let success = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(callback),
            LPARAM((&mut search as *mut MonitorSearch) as isize),
        )
        .as_bool()
    };
    if !success {
        return Err(format!(
            "не удалось перечислить мониторы Windows: {}",
            windows::core::Error::from_win32()
        ));
    }
    search
        .handle
        .ok_or_else(|| format!("выбранный монитор {source_id} больше не найден"))
}

struct MonitorSearch<'a> {
    source_id: &'a str,
    handle: Option<HMONITOR>,
}

fn monitor_id(handle: HMONITOR) -> Option<String> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if !unsafe {
        GetMonitorInfoW(
            handle,
            (&mut info as *mut MONITORINFOEXW).cast::<MONITORINFO>(),
        )
        .as_bool()
    } {
        return None;
    }
    Some(wide_string(&info.szDevice))
}

fn wide_string(value: &[u16]) -> String {
    let length = value
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..length])
}

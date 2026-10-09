//! Временные hooks для назначения кнопки; отмена future освобождает worker.

use super::super::super::key::PushToTalkKey;
use super::{input, recording::Recording};
use dioxus::prelude::{info, warn};
use futures_channel::oneshot;
use std::cell::RefCell;
use std::ptr::null_mut;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

type ResultSender = oneshot::Sender<Result<PushToTalkKey, String>>;
struct Context {
    recording: Recording,
    sender: Option<ResultSender>,
    stopped: Arc<AtomicBool>,
}
thread_local! { static CONTEXT: RefCell<Option<Context>> = const { RefCell::new(None) }; }
struct Cancellation(Arc<AtomicBool>);
impl Drop for Cancellation {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

/// Записывает одну кнопку и возвращает её после отпускания; ожидает не более 30 секунд.
///
/// # Errors
/// Сообщает об ошибке hook, завершении message loop или превышении времени ожидания.
pub(super) async fn record_binding() -> Result<PushToTalkKey, String> {
    let (sender, receiver) = oneshot::channel();
    let stopped = Arc::new(AtomicBool::new(false));
    let _cancellation = Cancellation(stopped.clone());
    std::thread::Builder::new().name("cheenhub-binding-recorder".into()).spawn(move || {
        CONTEXT.with(|context| *context.borrow_mut() = Some(Context { recording: Recording::default(), sender: Some(sender), stopped: stopped.clone() }));
        if let Err(error) = run(&stopped) {
            warn!(%error, kind = "binding_recording", "Windows push-to-talk binding recording failed");
            CONTEXT.with(|context| { if let Some(context) = context.borrow_mut().as_mut() && let Some(sender) = context.sender.take() { let _ = sender.send(Err(error)); } });
        }
        CONTEXT.with(|context| *context.borrow_mut() = None);
        info!("Windows push-to-talk binding recorder stopped");
    }).map_err(|error| { warn!(%error, "failed to spawn binding recorder"); "Не удалось начать назначение кнопки. Попробуйте ещё раз.".to_owned() })?;
    info!("Windows push-to-talk binding recording started");
    receiver
        .await
        .map_err(|_| "Назначение кнопки прервано. Попробуйте ещё раз.".to_owned())?
}

struct Hooks {
    keyboard: HHOOK,
    mouse: HHOOK,
    timer: usize,
}
impl Drop for Hooks {
    fn drop(&mut self) {
        unsafe {
            if !self.keyboard.is_null() {
                UnhookWindowsHookEx(self.keyboard);
            }
            if !self.mouse.is_null() {
                UnhookWindowsHookEx(self.mouse);
            }
            if self.timer != 0 {
                KillTimer(null_mut(), self.timer);
            }
        }
    }
}
fn run(stopped: &AtomicBool) -> Result<(), String> {
    let mut hooks = Hooks {
        keyboard: null_mut(),
        mouse: null_mut(),
        timer: 0,
    };
    let module = unsafe { GetModuleHandleW(null_mut()) };
    hooks.keyboard = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_event), module, 0) };
    hooks.mouse = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_event), module, 0) };
    hooks.timer = unsafe { SetTimer(null_mut(), 0, 100, None) };
    if hooks.keyboard.is_null() || hooks.mouse.is_null() || hooks.timer == 0 {
        warn!(error = %std::io::Error::last_os_error(), "failed to install Windows binding recorder hooks");
        return Err("Не удалось начать назначение кнопки. Попробуйте ещё раз.".into());
    }
    // Snapshot после установки hooks закрывает промежуток между проверкой и установкой.
    let held = (1..=254)
        .filter_map(PushToTalkKey::from_code)
        .filter(|key| unsafe { GetAsyncKeyState(i32::from(key.code())) < 0 })
        .collect();
    CONTEXT.with(|context| {
        if let Some(context) = context.borrow_mut().as_mut() {
            context.recording = Recording::with_initially_held(held);
        }
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut message: MSG = unsafe { std::mem::zeroed() };
    while !stopped.load(Ordering::Acquire) {
        let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if result <= 0 {
            return Err("Назначение кнопки прервано. Попробуйте ещё раз.".into());
        }
        if Instant::now() >= deadline {
            return Err("Кнопка не выбрана. Нажмите «Назначить» и попробуйте ещё раз.".into());
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}
fn receive(event: input::InputEvent) -> bool {
    CONTEXT.with(|context| {
        let mut context = context.borrow_mut();
        let Some(context) = context.as_mut() else {
            return false;
        };
        if context.stopped.load(Ordering::Acquire) {
            return false;
        }
        let captured = context.recording.captures(event.key, event.pressed);
        if let Some(key) = context.recording.input(event.key, event.pressed) {
            if let Some(sender) = context.sender.take() {
                let _ = sender.send(Ok(key));
            }
            context.stopped.store(true, Ordering::Release);
        }
        // Основные кнопки пропускаются, чтобы «Отменить» работало при любой настройке мыши.
        captured && !matches!(event.key.code(), 1 | 2)
    })
}
unsafe extern "system" fn keyboard_event(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let event = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
        if let Some(event) = input::keyboard(wparam as u32, event)
            && receive(event)
        {
            return 1;
        }
    }
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}
unsafe extern "system" fn mouse_event(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let event = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };
        if let Some(event) = input::mouse(wparam as u32, event.mouseData)
            && receive(event)
        {
            return 1;
        }
    }
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

//! Windows keyboard hook с отдельным message loop и восстановлением при потере событий.

use std::cell::RefCell;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use dioxus::prelude::{info, warn};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, KillTimer,
    LLKHF_EXTENDED, MSG, MSLLHOOKSTRUCT, SetTimer, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_TIMER,
};

mod input;
mod recorder;
mod recording;

use super::super::gate::Gate;
use super::super::key::PushToTalkKey;

const WATCHDOG_MS: u32 = 250;
const STALE_AFTER_MS: u64 = 1_000;

struct Shared {
    pressed: AtomicBool,
    epoch: AtomicU64,
    stopped: AtomicBool,
    heartbeat_ms: AtomicU64,
    origin: Instant,
    recording_generation: Arc<AtomicU64>,
}

struct HookContext {
    key: PushToTalkKey,
    gate: Gate,
    shared: Arc<Shared>,
}

thread_local! {
    static CONTEXT: RefCell<Option<HookContext>> = const { RefCell::new(None) };
}

/// Владеет hook worker; устаревший heartbeat закрывает передачу.
pub(in crate::features::microphone) struct Monitor {
    shared: Arc<Shared>,
}

impl Monitor {
    /// Запускает worker и ожидает первую успешную установку hook не более трёх секунд.
    ///
    /// # Errors
    /// Возвращает сообщение для пользователя при сбое запуска или недоступности hook.
    pub(in crate::features::microphone) fn start(
        key: PushToTalkKey,
        recording_generation: Arc<AtomicU64>,
    ) -> Result<Self, &'static str> {
        let shared = Arc::new(Shared {
            pressed: AtomicBool::new(false),
            epoch: AtomicU64::new(1),
            stopped: AtomicBool::new(false),
            heartbeat_ms: AtomicU64::new(0),
            origin: Instant::now(),
            recording_generation,
        });
        let (ready_tx, ready_rx) = mpsc::channel();
        let worker = shared.clone();
        if let Err(error) = thread::Builder::new()
            .name("cheenhub-push-to-talk".into())
            .spawn(move || {
                let mut ready = Some(ready_tx);
                CONTEXT.with(|context| {
                    *context.borrow_mut() = Some(HookContext {
                        key,
                        gate: Gate::default(),
                        shared: worker.clone(),
                    });
                });
                info!(
                    key_code = key.code(),
                    "Windows push-to-talk event loop started"
                );
                while !worker.stopped.load(Ordering::Acquire) {
                    run_event_loop(key, &worker, &mut ready);
                    close_gate();
                    if !worker.stopped.load(Ordering::Acquire) {
                        warn!(
                            kind = "hook_restart",
                            "Windows push-to-talk event loop retry scheduled"
                        );
                        thread::sleep(Duration::from_secs(1));
                    }
                }
                CONTEXT.with(|context| *context.borrow_mut() = None);
                info!("Windows push-to-talk event loop stopped");
            })
        {
            warn!(%error, kind = "hook_spawn", "failed to start Windows push-to-talk worker");
        }
        let monitor = Self { shared };
        if ready_rx.recv_timeout(Duration::from_secs(3)).is_err() {
            return Err(
                "Не удалось включить Push-to-talk. Выберите другой режим микрофона или попробуйте ещё раз.",
            );
        }
        Ok(monitor)
    }

    /// Возвращает поколение только при согласованном удержании и свежем heartbeat.
    pub(in crate::features::microphone) fn held_epoch(&self) -> Option<(u64, u64)> {
        let before = self.shared.epoch.load(Ordering::Acquire);
        let recording = self.shared.recording_generation.load(Ordering::Acquire);
        (recording.is_multiple_of(2)
            && self.pressed()
            && before == self.shared.epoch.load(Ordering::Acquire)
            && recording == self.shared.recording_generation.load(Ordering::Acquire))
        .then_some((before, recording))
    }

    /// Возвращает удержание только при свежем heartbeat работающего event loop.
    pub(in crate::features::microphone) fn pressed(&self) -> bool {
        let elapsed_ms = elapsed_ms(&self.shared);
        let heartbeat = self.shared.heartbeat_ms.load(Ordering::Acquire);
        elapsed_ms.saturating_sub(heartbeat) <= STALE_AFTER_MS
            && !self.shared.stopped.load(Ordering::Acquire)
            && self.shared.pressed.load(Ordering::Acquire)
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.shared.pressed.store(false, Ordering::Release);
        self.shared.stopped.store(true, Ordering::Release);
    }
}

fn elapsed_ms(shared: &Shared) -> u64 {
    shared
        .origin
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn close_gate() {
    CONTEXT.with(|context| {
        if let Some(context) = context.borrow_mut().as_mut() {
            context.shared.epoch.fetch_add(1, Ordering::AcqRel);
            context.gate.failed();
            context.shared.pressed.store(false, Ordering::Release);
        }
    });
}

fn recovered(held: bool) {
    CONTEXT.with(|context| {
        if let Some(context) = context.borrow_mut().as_mut() {
            context.shared.epoch.fetch_add(1, Ordering::AcqRel);
            context.gate.recover(held);
            context
                .shared
                .pressed
                .store(context.gate.active(), Ordering::Release);
        }
    });
}

fn physical_key_held(key: PushToTalkKey) -> bool {
    // Старший бит отражает текущее удержание; младший бит не является надёжным событием.
    unsafe { GetAsyncKeyState(i32::from(key.code())) < 0 }
}

fn install_hook(key: PushToTalkKey) -> HHOOK {
    // Callback принадлежит этому модулю, а message loop остаётся на вызывающем потоке.
    unsafe {
        SetWindowsHookExW(
            if key.is_mouse() {
                WH_MOUSE_LL
            } else {
                WH_KEYBOARD_LL
            },
            if key.is_mouse() {
                Some(mouse_event)
            } else {
                Some(keyboard_event)
            },
            GetModuleHandleW(null_mut()),
            0,
        )
    }
}

fn run_event_loop(key: PushToTalkKey, shared: &Shared, ready: &mut Option<mpsc::Sender<()>>) {
    let mut hook = install_hook(key);
    if hook.is_null() {
        warn!(error = %std::io::Error::last_os_error(), kind = "hook_install", "Windows push-to-talk hook installation failed");
        return;
    }
    // Thread timer обеспечивает watchdog и завершение даже без клавиатурных событий.
    let timer = unsafe { SetTimer(null_mut(), 0, WATCHDOG_MS, None) };
    if timer == 0 {
        warn!(error = %std::io::Error::last_os_error(), kind = "hook_timer", "Windows push-to-talk watchdog setup failed");
        unsafe {
            UnhookWindowsHookEx(hook);
        }
        return;
    }
    recovered(physical_key_held(key));
    shared
        .heartbeat_ms
        .store(elapsed_ms(shared), Ordering::Release);
    if let Some(ready) = ready.take() {
        let _ = ready.send(());
    }
    info!(key_code = key.code(), "Windows push-to-talk hook installed");
    let mut message: MSG = unsafe { std::mem::zeroed() };
    loop {
        let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if result <= 0 {
            if result < 0 {
                warn!(error = %std::io::Error::last_os_error(), kind = "hook_messages", "Windows push-to-talk message loop failed");
            }
            break;
        }
        if shared.stopped.load(Ordering::Acquire) {
            break;
        }
        if message.message == WM_TIMER && message.wParam == timer {
            let physical = physical_key_held(key);
            let stale = elapsed_ms(shared)
                .saturating_sub(shared.heartbeat_ms.load(Ordering::Acquire))
                > STALE_AFTER_MS;
            if hook.is_null() || physical != shared.pressed.load(Ordering::Acquire) || stale {
                // Windows может удалить hook без уведомления после timeout callback.
                close_gate();
                if !hook.is_null() {
                    unsafe {
                        UnhookWindowsHookEx(hook);
                    }
                }
                hook = install_hook(key);
                if hook.is_null() {
                    warn!(error = %std::io::Error::last_os_error(), kind = "hook_recovery", "Windows push-to-talk hook recovery failed; transmission closed");
                } else {
                    recovered(physical_key_held(key));
                    warn!(
                        kind = "hook_recovered",
                        "Windows push-to-talk hook restored after input state mismatch"
                    );
                }
            }
            shared
                .heartbeat_ms
                .store(elapsed_ms(shared), Ordering::Release);
        } else {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
    close_gate();
    unsafe {
        KillTimer(null_mut(), timer);
        if !hook.is_null() {
            UnhookWindowsHookEx(hook);
        }
    }
}

fn receive(event: input::InputEvent) {
    CONTEXT.with(|context| {
        if let Some(context) = context.borrow_mut().as_mut()
            && event.key == context.key
        {
            let previous = context.gate.active();
            context.gate.event(event.pressed);
            if previous != context.gate.active() {
                context.shared.epoch.fetch_add(1, Ordering::AcqRel);
            }
            context
                .shared
                .pressed
                .store(context.gate.active(), Ordering::Release);
        }
    });
}
unsafe extern "system" fn keyboard_event(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let event = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
        if let Some(event) = input::keyboard(wparam as u32, event) {
            receive(event);
        }
    }
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}
unsafe extern "system" fn mouse_event(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let event = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };
        if let Some(event) = input::mouse(wparam as u32, event.mouseData) {
            receive(event);
        }
    }
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

fn key_code(vk: u32, scan: u32, flags: u32) -> u32 {
    match vk {
        0x10 => {
            if scan == 0x36 {
                0xa1
            } else {
                0xa0
            }
        }
        0x11 => {
            if flags & LLKHF_EXTENDED != 0 {
                0xa3
            } else {
                0xa2
            }
        }
        0x12 => {
            if flags & LLKHF_EXTENDED != 0 {
                0xa5
            } else {
                0xa4
            }
        }
        _ => vk,
    }
}

/// Глобальный hook доступен в Windows desktop.
pub(in crate::features::microphone) fn supported() -> bool {
    true
}
/// Windows не требует сообщения о платформенной недоступности.
pub(in crate::features::microphone) fn unsupported_reason() -> &'static str {
    ""
}

#[cfg(test)]
mod tests;

/// Возвращает название выбранной кнопки из системной раскладки.
pub(in crate::features::microphone) fn key_label(key: PushToTalkKey) -> String {
    input::key_label(key)
}

/// Записывает кнопку глобальными hooks без зависимости UI от Windows API.
///
/// # Errors
/// Возвращает ошибку запуска recorder или истечения времени ожидания.
pub(in crate::features::microphone) async fn record_binding() -> Result<PushToTalkKey, String> {
    recorder::record_binding().await
}

//! Windows-реализация единственного экземпляра через именованные mutex и event.

use std::ptr::null;
use std::sync::{Mutex, OnceLock};

use dioxus::desktop::use_window;
use dioxus::prelude::*;
use futures_channel::mpsc::{UnboundedReceiver, unbounded};
use futures_util::StreamExt;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, ERROR_SUCCESS, GetLastError, HANDLE, SetLastError,
    WAIT_OBJECT_0,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, INFINITE, SetEvent, WaitForSingleObject,
};

const MUTEX_NAME: &str = r"Local\ru.cheenhub.client.single-instance.v1";
const ACTIVATION_EVENT_NAME: &str = r"Local\ru.cheenhub.client.activate.v1";

static INSTANCE_GUARD: OnceLock<InstanceGuard> = OnceLock::new();
static ACTIVATION_RECEIVER: Mutex<Option<UnboundedReceiver<()>>> = Mutex::new(None);

/// Регистрирует первый экземпляр или уведомляет его о повторном запуске.
pub(crate) fn prepare() -> Result<bool, String> {
    let event_name = wide_null(ACTIVATION_EVENT_NAME);
    let event = unsafe { CreateEventW(null(), 0, 0, event_name.as_ptr()) };
    if event.is_null() {
        return Err(format!(
            "Не удалось создать событие активации CheenHub: {}",
            std::io::Error::last_os_error()
        ));
    }

    let mutex_name = wide_null(MUTEX_NAME);
    unsafe {
        SetLastError(ERROR_SUCCESS);
    }
    let mutex = unsafe { CreateMutexW(null(), 0, mutex_name.as_ptr()) };
    if mutex.is_null() {
        let error = std::io::Error::last_os_error();
        unsafe {
            CloseHandle(event);
        }
        return Err(format!(
            "Не удалось создать mutex единственного экземпляра CheenHub: {error}"
        ));
    }
    let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;

    if already_exists {
        let notified = unsafe { SetEvent(event) };
        let notification_error = (notified == 0).then(std::io::Error::last_os_error);
        unsafe {
            CloseHandle(mutex);
            CloseHandle(event);
        }
        if let Some(error) = notification_error {
            return Err(format!(
                "Не удалось активировать запущенный экземпляр CheenHub: {error}"
            ));
        }
        info!("forwarded activation request to existing CheenHub instance");
        return Ok(false);
    }

    let guard = InstanceGuard { mutex, event };
    let (sender, receiver) = unbounded();
    *ACTIVATION_RECEIVER
        .lock()
        .map_err(|_| "Не удалось сохранить канал активации CheenHub.".to_owned())? = Some(receiver);

    let event_address = guard.event as usize;
    std::thread::Builder::new()
        .name("cheenhub-instance-activation".to_owned())
        .spawn(move || {
            let event = event_address as HANDLE;
            loop {
                let wait_result = unsafe { WaitForSingleObject(event, INFINITE) };
                if wait_result != WAIT_OBJECT_0 {
                    error!(wait_result, "CheenHub activation event wait failed");
                    break;
                }
                if sender.unbounded_send(()).is_err() {
                    debug!("CheenHub activation receiver was closed");
                    break;
                }
                debug!("received request to activate primary CheenHub instance");
            }
        })
        .map_err(|error| format!("Не удалось запустить обработчик активации CheenHub: {error}"))?;

    INSTANCE_GUARD
        .set(guard)
        .map_err(|_| "Экземпляр CheenHub уже был подготовлен в текущем процессе.".to_owned())?;
    info!("registered primary CheenHub desktop instance");
    Ok(true)
}

/// Передаёт UI-потребителю поток запросов на показ окна.
fn take_activation_receiver() -> Option<UnboundedReceiver<()>> {
    ACTIVATION_RECEIVER
        .lock()
        .ok()
        .and_then(|mut receiver| receiver.take())
}

/// Показывает главное окно, когда второй процесс запрашивает активацию.
#[component]
pub(crate) fn SingleInstanceEffects() -> Element {
    let window = use_window();
    use_future(move || {
        let window = window.clone();
        async move {
            let Some(mut receiver) = take_activation_receiver() else {
                return;
            };
            while receiver.next().await.is_some() {
                window.set_visible(true);
                window.set_minimized(false);
                window.set_focus();
                info!("activated primary CheenHub window after repeated launch");
            }
        }
    });

    rsx! {}
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

struct InstanceGuard {
    mutex: HANDLE,
    event: HANDLE,
}

unsafe impl Send for InstanceGuard {}
unsafe impl Sync for InstanceGuard {}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.mutex);
            CloseHandle(self.event);
        }
    }
}

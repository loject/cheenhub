//! GlobalShortcuts Portal: системное назначение и fail-closed источник удержания Linux.

/// Сохранённая Linux-привязка без Windows Virtual-Key-кодов.
pub(crate) mod binding;
mod identity;
mod session;
mod state;
mod updates;
mod worker;

use crate::features::microphone::push_to_talk::Update;
use binding::PushToTalkKey;
use dioxus::prelude::warn;
use state::State;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use tokio::sync::{mpsc, oneshot, watch};

/// Внутреннее состояние Linux Portal и готовности capture.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Status {
    /// Portal доступен, но активной PTT-сессии нет.
    Available,
    /// Требуется назначение сочетания, передача остановлена.
    BindingRequired,
    /// Сессия готова к новому событию Activated.
    Ready,
    /// Ошибка закрыла gate и требует реакции пользователя.
    Failed(String),
}

static SERVICE: Mutex<Option<Arc<Service>>> = Mutex::new(None);

struct Service {
    state: State,
    available: AtomicBool,
    users: AtomicUsize,
    commands: mpsc::UnboundedSender<Command>,
    stopped: AtomicBool,
    updates: watch::Sender<Update>,
}

enum Command {
    Ensure,
    Assign(oneshot::Sender<Result<PushToTalkKey, String>>),
    FinishAssignment,
    Probe,
    Release,
    Shutdown,
}

fn service() -> Arc<Service> {
    let mut slot = SERVICE.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(service) = slot.as_ref()
        && !service.stopped.load(Ordering::Acquire)
    {
        return service.clone();
    }
    let (commands, receiver) = mpsc::unbounded_channel();
    // Подписка provider переживает замену аварийно завершившегося worker.
    let updates = slot
        .as_ref()
        .map(|previous| previous.updates.clone())
        .unwrap_or_else(|| watch::channel(updates::initial()).0);
    let restart_capture = updates.borrow().restart_capture;
    let mut initial = updates::initial();
    initial.restart_capture = restart_capture;
    updates.send_replace(initial);
    let service = Arc::new(Service {
        state: State::default(),
        available: AtomicBool::new(false),
        users: AtomicUsize::new(0),
        commands,
        stopped: AtomicBool::new(false),
        updates,
    });
    let worker_service = service.clone();
    if let Err(error) = std::thread::Builder::new()
        .name("cheenhub-portal-ptt".into())
        .spawn(move || {
            let _exit = WorkerExit(worker_service.clone());
            match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime.block_on(worker::run(worker_service.clone(), receiver)),
                Err(error) => warn!(%error, kind = "portal_runtime", "Linux PTT runtime failed"),
            }
        })
    {
        warn!(%error, kind = "portal_spawn", "Linux PTT worker failed to start");
        service.available(false);
        service.publish(Status::Failed("Не удалось запустить системное управление Push-to-talk. Попробуйте проверить доступность снова.".into()));
        service.stopped.store(true, Ordering::Release);
    }
    *slot = Some(service.clone());
    service
}

/// Владеет участием capture в portal-сессии и проверкой поколения назначения.
pub(in crate::features::microphone) struct Monitor {
    service: Arc<Service>,
    recording_generation: Arc<AtomicU64>,
    released: AtomicBool,
}
impl Monitor {
    /// Подключает capture к portal event loop; до Activated gate остаётся закрытым.
    ///
    /// # Errors
    /// Остановленный worker не позволяет зарегистрировать capture lease.
    pub(in crate::features::microphone) fn start(
        _: PushToTalkKey,
        recording_generation: Arc<AtomicU64>,
    ) -> Result<Self, &'static str> {
        Self::attach(service(), recording_generation)
    }

    fn attach(
        service: Arc<Service>,
        recording_generation: Arc<AtomicU64>,
    ) -> Result<Self, &'static str> {
        if service.stopped.load(Ordering::Acquire) {
            return Err(unsupported_reason());
        }
        service.users.fetch_add(1, Ordering::AcqRel);
        if service.commands.send(Command::Ensure).is_err() {
            service.users.fetch_sub(1, Ordering::AcqRel);
            service.state.fail();
            return Err(unsupported_reason());
        }
        service.capture_started();
        Ok(Self {
            service,
            recording_generation,
            released: AtomicBool::new(false),
        })
    }
    /// Возвращает epoch только при исправной сессии, свежем watchdog и отсутствии назначения.
    pub(in crate::features::microphone) fn held_epoch(&self) -> Option<(u64, u64)> {
        if self.released.load(Ordering::Acquire) || self.service.stopped.load(Ordering::Acquire) {
            return None;
        }
        let epoch = self.service.state.held_epoch(&self.recording_generation);
        (!self.released.load(Ordering::Acquire) && !self.service.stopped.load(Ordering::Acquire))
            .then_some(epoch)
            .flatten()
    }
}
impl Monitor {
    /// Отзывает capture lease сразу, даже если queued frames ещё удерживают Arc monitor.
    pub(super) fn stop(&self) {
        if !self.released.swap(true, Ordering::AcqRel)
            && self.service.users.fetch_sub(1, Ordering::AcqRel) == 1
        {
            self.service.state.fail();
            let _ = self.service.commands.send(Command::Release);
        }
    }
}
impl Drop for Monitor {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Возвращает последнюю фактическую проверку portal, без D-Bus-вызова на UI-потоке.
pub(in crate::features::microphone) fn supported() -> bool {
    service().available.load(Ordering::Acquire)
}

/// Проверяет portal асинхронно; отсутствие поддержки не разрешает передачу.
pub(super) async fn availability() -> bool {
    let service = service();
    let mut updates = service.updates.subscribe();
    loop {
        match updates.borrow_and_update().availability {
            Some(true) => return true,
            Some(false) => return false,
            None => {}
        }
        if updates.changed().await.is_err() {
            return false;
        }
    }
}

/// Объясняет отсутствие системной возможности глобального назначения.
pub(in crate::features::microphone) fn unsupported_reason() -> &'static str {
    "В этой среде рабочего стола глобальный Push-to-talk недоступен. Выберите другой режим микрофона или используйте среду с поддержкой системных глобальных сочетаний."
}

/// Открывает системное назначение; отмена закрывает сессию и отзывает удержание.
///
/// # Errors
/// Возвращает понятное сообщение при отмене, недоступности portal или ошибке сессии.
pub(in crate::features::microphone) async fn record_binding() -> Result<PushToTalkKey, String> {
    let service = service();
    service.state.fail();
    let (reply, result) = oneshot::channel();
    service
        .commands
        .send(Command::Assign(reply))
        .map_err(|_| unsupported_reason().to_owned())?;
    result
        .await
        .map_err(|_| "Системное назначение прервано. Попробуйте ещё раз.".to_owned())?
}

/// Обслуживает реактивную подписку provider; worker никогда не меняет Dioxus signal напрямую.
pub(super) async fn observe_updates(mut on_update: impl FnMut(Update)) {
    let service = service();
    let mut updates = service.updates.subscribe();
    loop {
        on_update(updates.borrow_and_update().clone());
        if updates.changed().await.is_err() {
            break;
        }
    }
}

/// Завершает явное переназначение без ожидания отсутствующего сигнала отмены portal.
pub(super) fn finish_assignment() {
    let _ = service().commands.send(Command::FinishAssignment);
}

/// Повторяет probe по действию пользователя без назначения и захвата сочетаний.
pub(super) fn retry_availability() {
    let service = service();
    service.publish(Status::Available);
    let _ = service.commands.send(Command::Probe);
}

/// Закрывает gate сразу и завершает worker с освобождением portal-сессии.
pub(super) fn shutdown() {
    if let Some(service) = SERVICE
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take()
    {
        service.state.fail();
        service.stopped.store(true, Ordering::Release);
        let _ = service.commands.send(Command::Shutdown);
    }
}

#[cfg(test)]
mod tests;

struct WorkerExit(Arc<Service>);
impl Drop for WorkerExit {
    fn drop(&mut self) {
        self.0.state.fail();
        self.0.available(false);
        if !self.0.stopped.load(Ordering::Acquire) {
            if self.0.users.load(Ordering::Acquire) > 0 {
                self.0
                    .updates
                    .send_modify(|update| update.restart_capture = true);
            }
            self.0.publish(Status::Failed("Системное управление Push-to-talk прервано. Откройте настройки и проверьте доступность снова.".into()));
        }
        self.0.stopped.store(true, Ordering::Release);
        if std::thread::panicking() {
            warn!(
                kind = "portal_worker_panic",
                "Linux PTT worker panicked; gate closed"
            );
        }
    }
}

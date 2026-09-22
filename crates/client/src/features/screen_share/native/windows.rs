//! Непрерывный Windows Graphics Capture физических мониторов.

use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use dioxus::core::{Runtime, ScopeId, current_scope_id};
use dioxus::logger::tracing::{info, warn};
use dioxus::prelude::spawn;
use futures_channel::{mpsc, oneshot};
use futures_util::future::LocalBoxFuture;
use futures_util::{FutureExt, StreamExt};

use super::super::backend::{
    ScreenShareBackend, ScreenShareCallbacks, ScreenShareCaptureSource, ScreenShareError,
    ScreenShareSession, ScreenShareStartRequest,
};

mod capture;
use capture::{WorkerEvent, capture_worker};

const FIRST_FRAME_TIMEOUT: Duration = Duration::from_secs(5);
const FRAME_POOL_BUFFER_COUNT: i32 = 2;
const FRAME_REPORT_INTERVAL: Duration = Duration::from_secs(1);

/// Реализация непрерывного захвата выбранного физического монитора через WGC.
pub(crate) struct WindowsScreenShareBackend;

impl ScreenShareBackend for WindowsScreenShareBackend {
    fn start(
        &self,
        request: ScreenShareStartRequest,
        callbacks: ScreenShareCallbacks,
    ) -> LocalBoxFuture<'static, Result<std::rc::Rc<dyn ScreenShareSession>, ScreenShareError>>
    {
        async move {
            let ScreenShareCaptureSource::Selected { source_id, target } = request.source else {
                return Err(ScreenShareError::new(
                    "Windows backend требует выбранный физический монитор.",
                ));
            };

            info!(
                source_id = %source_id,
                target_width = target.width,
                target_height = target.height,
                target_fps = target.max_fps,
                "Windows screen capture requested"
            );

            let control = Arc::new(CaptureControl::default());
            let (startup_sender, startup_receiver) = oneshot::channel();
            let (runtime_sender, runtime_receiver) = mpsc::unbounded();
            spawn_runtime_event_relay(current_scope_id(), runtime_receiver, callbacks);

            let worker_control = control.clone();
            let worker_source_id = source_id.clone();
            let worker = thread::Builder::new()
                .name("cheenhub-screen-capture".to_owned())
                .spawn(move || {
                    capture_worker(
                        worker_source_id,
                        worker_control,
                        startup_sender,
                        runtime_sender,
                    )
                })
                .map_err(|error| {
                    ScreenShareError::new(format!(
                        "не удалось запустить поток Windows Graphics Capture: {error}"
                    ))
                })?;
            let mut startup_guard = WindowsStartupGuard::new(control.clone(), worker);

            match tokio::time::timeout(FIRST_FRAME_TIMEOUT, startup_receiver).await {
                Ok(Ok(Ok(startup))) => {
                    info!(
                        source_id = %source_id,
                        native_width = startup.width,
                        native_height = startup.height,
                        "Windows screen capture first valid frame received"
                    );
                    let (control, worker) = startup_guard.disarm();
                    Ok(std::rc::Rc::new(WindowsScreenShareSession {
                        control,
                        worker: std::cell::RefCell::new(Some(worker)),
                        stopped: std::cell::Cell::new(false),
                    })
                        as std::rc::Rc<dyn ScreenShareSession>)
                }
                Ok(Ok(Err(error))) => {
                    startup_guard.request_stop_and_join().await?;
                    Err(ScreenShareError::new(error))
                }
                Ok(Err(_)) => {
                    startup_guard.request_stop_and_join().await?;
                    Err(ScreenShareError::new(
                        "поток Windows Graphics Capture завершился до первого кадра",
                    ))
                }
                Err(_) => {
                    warn!(
                        source_id = %source_id,
                        timeout_seconds = FIRST_FRAME_TIMEOUT.as_secs(),
                        "Windows Graphics Capture first frame timeout"
                    );
                    startup_guard.request_stop_and_join().await?;
                    Err(ScreenShareError::new(
                        "Windows Graphics Capture не получил первый кадр за 5 секунд",
                    ))
                }
            }
        }
        .boxed_local()
    }
}

struct WindowsScreenShareSession {
    control: Arc<CaptureControl>,
    worker: std::cell::RefCell<Option<JoinHandle<()>>>,
    stopped: std::cell::Cell<bool>,
}

struct WindowsStartupGuard {
    control: Arc<CaptureControl>,
    worker: Option<JoinHandle<()>>,
    armed: bool,
}

impl WindowsStartupGuard {
    fn new(control: Arc<CaptureControl>, worker: JoinHandle<()>) -> Self {
        Self {
            control,
            worker: Some(worker),
            armed: true,
        }
    }

    fn disarm(mut self) -> (Arc<CaptureControl>, JoinHandle<()>) {
        self.armed = false;
        let worker = self
            .worker
            .take()
            .expect("Windows startup guard должен владеть worker");
        (self.control.clone(), worker)
    }

    async fn request_stop_and_join(&mut self) -> Result<(), ScreenShareError> {
        self.control.request_stop();
        self.armed = false;
        if let Some(worker) = self.worker.take() {
            join_worker(worker).await?;
        }
        Ok(())
    }
}

impl Drop for WindowsStartupGuard {
    fn drop(&mut self) {
        if self.armed {
            self.control.request_stop();
        }
    }
}

impl Drop for WindowsScreenShareSession {
    fn drop(&mut self) {
        self.control.request_stop();
    }
}

impl ScreenShareSession for WindowsScreenShareSession {
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), ScreenShareError>> {
        if self.stopped.replace(true) {
            return async { Ok(()) }.boxed_local();
        }

        info!("stopping Windows screen capture explicitly");
        self.control.request_stop();
        let worker = self.worker.borrow_mut().take();
        async move {
            if let Some(worker) = worker {
                join_worker(worker).await?;
            }
            info!("Windows screen capture stopped and cleaned up");
            Ok(())
        }
        .boxed_local()
    }
}

async fn join_worker(worker: JoinHandle<()>) -> Result<(), ScreenShareError> {
    tokio::task::spawn_blocking(move || worker.join())
        .await
        .map_err(|error| ScreenShareError::new(format!("поток захвата не завершился: {error}")))?
        .map_err(|_| ScreenShareError::new("поток захвата завершился с panic"))
}

fn spawn_runtime_event_relay(
    provider_scope: ScopeId,
    mut events: mpsc::UnboundedReceiver<WorkerEvent>,
    callbacks: ScreenShareCallbacks,
) {
    let on_ended = callbacks.on_ended;
    let on_error = callbacks.on_error;
    Runtime::current().in_scope(provider_scope, || {
        spawn(async move {
            while let Some(event) = events.next().await {
                match event {
                    WorkerEvent::Ended => on_ended(),
                    WorkerEvent::Error(message) => on_error(ScreenShareError::new(message)),
                }
            }
        })
    });
}

#[derive(Default)]
struct CaptureControl {
    state: Mutex<CaptureControlState>,
    wake: Condvar,
}

#[derive(Default)]
struct CaptureControlState {
    stop_requested: bool,
    source_closed: bool,
    frame_generation: u64,
}

struct CaptureControlSnapshot {
    stop_requested: bool,
    source_closed: bool,
    frame_generation: u64,
}

impl CaptureControl {
    fn frame_arrived(&self) {
        let mut state = lock_control(&self.state);
        state.frame_generation = state.frame_generation.saturating_add(1);
        self.wake.notify_one();
    }

    fn source_closed(&self) {
        let mut state = lock_control(&self.state);
        state.source_closed = true;
        self.wake.notify_one();
    }

    fn request_stop(&self) {
        let mut state = lock_control(&self.state);
        state.stop_requested = true;
        self.wake.notify_one();
    }

    fn wait_for_change(&self, last_generation: u64) -> CaptureControlSnapshot {
        let mut state = lock_control(&self.state);
        while !state.stop_requested
            && !state.source_closed
            && state.frame_generation == last_generation
        {
            state = self
                .wake
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        CaptureControlSnapshot {
            stop_requested: state.stop_requested,
            source_closed: state.source_closed,
            frame_generation: state.frame_generation,
        }
    }
}

fn lock_control<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

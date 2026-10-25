//! Временная loopback-раздача WebM с ограничением памяти и единственным читателем.

use std::{convert::Infallible, io, net::TcpListener, sync::Arc};

use axum::{
    Router,
    body::{Body, Bytes},
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use dioxus::logger::tracing::{debug, warn};
use futures_util::stream;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore, mpsc, watch};

const MAX_QUEUED_BYTES: usize = 8 * 1024 * 1024;
const MAX_QUEUED_CHUNKS: usize = 128;

struct Chunk {
    bytes: Vec<u8>,
    _budget: OwnedSemaphorePermit,
}

struct Queue {
    sender: mpsc::Sender<Chunk>,
    budget: Arc<Semaphore>,
}

impl Queue {
    fn new() -> (Self, mpsc::Receiver<Chunk>) {
        let (sender, receiver) = mpsc::channel(MAX_QUEUED_CHUNKS);
        (
            Self {
                sender,
                budget: Arc::new(Semaphore::new(MAX_QUEUED_BYTES)),
            },
            receiver,
        )
    }

    fn send(&self, bytes: Vec<u8>) -> Result<(), String> {
        if bytes.is_empty() || bytes.len() > MAX_QUEUED_BYTES {
            return Err("Недопустимый размер фрагмента WebM".into());
        }
        let budget = self
            .budget
            .clone()
            .try_acquire_many_owned(bytes.len() as u32)
            .map_err(|_| "Плеер не успевает получать видео: превышен лимит памяти")?;
        self.sender
            .try_send(Chunk {
                bytes,
                _budget: budget,
            })
            .map_err(|_| "Плеер остановлен или не успевает получать видеокадры".into())
    }
}

#[derive(Clone)]
struct StreamState {
    receiver: Arc<Mutex<Option<mpsc::Receiver<Chunk>>>>,
    shutdown: watch::Receiver<bool>,
}

/// Один временный URL видео; владелец закрывает его вместе с плиткой участника.
pub(super) struct LocalVideoStream {
    url: String,
    queue: Option<Queue>,
    shutdown: watch::Sender<bool>,
}

impl LocalVideoStream {
    /// Привязывает сервер только к IPv4 loopback и создаёт непредсказуемый путь.
    pub(super) fn new() -> io::Result<Self> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        let address = listener.local_addr()?;
        listener.set_nonblocking(true)?;
        let listener = tokio::net::TcpListener::from_std(listener)?;
        let path = format!("/{}.webm", uuid::Uuid::new_v4().simple());
        let url = format!("http://{address}{path}");
        let (queue, receiver) = Queue::new();
        let (shutdown, mut stopped) = watch::channel(false);
        let state = StreamState {
            receiver: Arc::new(Mutex::new(Some(receiver))),
            shutdown: stopped.clone(),
        };
        let router = Router::new()
            .route(&path, get(video_body).head(video_head))
            .with_state(state);
        tokio::spawn(async move {
            let result = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = stopped.changed().await;
                })
                .await;
            if let Err(error) = result {
                warn!(%error, %address, "Linux video loopback server failed");
            }
            debug!(%address, "Linux video loopback server stopped");
        });
        debug!(%address, "Linux video loopback server listening");
        Ok(Self {
            url,
            queue: Some(queue),
            shutdown,
        })
    }

    /// Возвращает локальный URL; случайный путь не следует записывать в логи.
    pub(super) fn url(&self) -> &str {
        &self.url
    }

    /// Передаёт законченный фрагмент WebM без блокировки UI и без роста очереди.
    pub(super) fn send(&self, bytes: Vec<u8>) -> Result<(), String> {
        self.queue.as_ref().ok_or("Поток видео закрыт")?.send(bytes)
    }

    /// Прерывает HTTP body, освобождает очередь и прекращает приём соединений.
    pub(super) fn close(&mut self) {
        self.queue.take();
        let _ = self.shutdown.send(true);
    }
}

impl Drop for LocalVideoStream {
    fn drop(&mut self) {
        self.close();
    }
}

fn video_headers() -> [(header::HeaderName, &'static str); 3] {
    [
        (header::CONTENT_TYPE, "video/webm"),
        (header::CACHE_CONTROL, "no-store"),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
    ]
}

async fn video_head() -> impl IntoResponse {
    video_headers()
}

async fn video_body(State(state): State<StreamState>) -> Response {
    let Some(receiver) = state.receiver.lock().await.take() else {
        return StatusCode::GONE.into_response();
    };
    let body = stream::unfold(
        (receiver, state.shutdown),
        |(mut receiver, mut shutdown)| async move {
            if *shutdown.borrow() {
                return None;
            }
            let chunk = tokio::select! {
                biased;
                _ = shutdown.changed() => return None,
                chunk = receiver.recv() => chunk?,
            };
            Some((
                Ok::<_, Infallible>(Bytes::from(chunk.bytes)),
                (receiver, shutdown),
            ))
        },
    );
    (video_headers(), Body::from_stream(body)).into_response()
}

#[cfg(test)]
mod tests;

//! Настройка логирования, трассировки и оперативного журнала бэкенда.

use std::{
    collections::VecDeque,
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::anyhow;
use cheenhub_contracts::rest::HostLogEntry;
use chrono::{SecondsFormat, Utc};
use tokio::sync::broadcast;
use tracing::{
    Event, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{
    EnvFilter, Layer, fmt as tracing_fmt,
    layer::{Context, SubscriberExt},
    util::SubscriberInitExt,
};

const HOST_LOG_CAPACITY: usize = 3_000;
const HOST_LOG_BROADCAST_CAPACITY: usize = 1_024;

/// Хранит последние записи журнала и рассылает новые подписчикам.
pub(crate) struct HostLogHub {
    entries: Mutex<VecDeque<HostLogEntry>>,
    sender: broadcast::Sender<HostLogEntry>,
    next_id: AtomicU64,
    capacity: usize,
}

impl HostLogHub {
    fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(HOST_LOG_BROADCAST_CAPACITY);
        Self {
            entries: Mutex::new(VecDeque::with_capacity(capacity)),
            sender,
            next_id: AtomicU64::new(1),
            capacity,
        }
    }

    /// Возвращает последние записи в хронологическом порядке.
    pub(crate) fn snapshot(&self, limit: usize) -> Vec<HostLogEntry> {
        let entries = match self.entries.lock() {
            Ok(entries) => entries,
            Err(poisoned) => poisoned.into_inner(),
        };
        let start = entries.len().saturating_sub(limit);
        entries.iter().skip(start).cloned().collect()
    }

    /// Создаёт realtime-подписку на новые записи.
    pub(crate) fn subscribe(&self) -> broadcast::Receiver<HostLogEntry> {
        self.sender.subscribe()
    }

    fn push(&self, level: String, target: String, message: String, fields: Vec<String>) {
        let entry = HostLogEntry {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            level,
            target,
            message,
            fields,
        };

        {
            let mut entries = match self.entries.lock() {
                Ok(entries) => entries,
                Err(poisoned) => poisoned.into_inner(),
            };
            while entries.len() >= self.capacity {
                entries.pop_front();
            }
            entries.push_back(entry.clone());
        }

        // Отсутствие активных подписчиков является нормальным состоянием.
        let _ = self.sender.send(entry);
    }
}

impl Default for HostLogHub {
    fn default() -> Self {
        Self::new(HOST_LOG_CAPACITY)
    }
}

#[derive(Clone)]
struct HostLogLayer {
    hub: Arc<HostLogHub>,
}

impl<S> Layer<S> for HostLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = HostLogVisitor::default();
        event.record(&mut visitor);

        let message = visitor
            .message
            .unwrap_or_else(|| metadata.name().to_owned());

        self.hub.push(
            metadata.level().as_str().to_owned(),
            metadata.target().to_owned(),
            message,
            visitor.fields,
        );
    }
}

#[derive(Default)]
struct HostLogVisitor {
    message: Option<String>,
    fields: Vec<String>,
}

impl HostLogVisitor {
    fn record_value(&mut self, field: &Field, value: String) {
        if field.name() == "message" {
            self.message = Some(value);
            return;
        }

        let value = if is_sensitive_field(field.name()) {
            "[REDACTED]".to_owned()
        } else {
            value
        };
        self.fields.push(format!("{}={value}", field.name()));
    }
}

impl Visit for HostLogVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_value(field, value.to_owned());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.record_value(field, format!("{value:?}"));
    }
}

fn is_sensitive_field(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "password",
        "access_token",
        "refresh_token",
        "authorization",
        "client_secret",
        "secret_key",
        "cookie",
        "session_token",
        "oauth_code",
    ]
    .iter()
    .any(|needle| name.contains(needle))
}

/// Инициализирует трассировку и возвращает оперативный журнал процесса.
pub(crate) fn init(filter: &str) -> anyhow::Result<Arc<HostLogHub>> {
    let hub = Arc::new(HostLogHub::new(HOST_LOG_CAPACITY));

    tracing_subscriber::registry()
        .with(EnvFilter::new(filter))
        .with(tracing_fmt::layer())
        .with(HostLogLayer { hub: hub.clone() })
        .try_init()
        .map_err(|error| anyhow!("failed to initialize tracing subscriber: {error}"))?;

    Ok(hub)
}

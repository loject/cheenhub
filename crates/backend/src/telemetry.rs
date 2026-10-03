//! Настройка логирования, трассировки и оперативного журнала бэкенда.

use std::{
    collections::VecDeque,
    fmt,
    sync::{
        Arc, Mutex, OnceLock,
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
    reload::Handle as ReloadHandle,
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
///
/// Фильтр оборачивается в перезагружаемый слой: [`set_log_level`] меняет
/// уровень на живом процессе без перезапуска. `filter` сохраняется как
/// фильтр запуска, к которому возвращает сброс настройки хоста.
pub(crate) fn init(filter: &str) -> anyhow::Result<Arc<HostLogHub>> {
    let hub = Arc::new(HostLogHub::new(HOST_LOG_CAPACITY));
    let (layer, reload_handle) = tracing_subscriber::reload::Layer::new(EnvFilter::new(filter));

    tracing_subscriber::registry()
        .with(layer)
        .with(tracing_fmt::layer())
        .with(HostLogLayer { hub: hub.clone() })
        .try_init()
        .map_err(|error| anyhow!("failed to initialize tracing subscriber: {error}"))?;

    let _ = STARTUP_FILTER.set(filter.to_owned());
    install_reload_filter(reload_handle);

    Ok(hub)
}

/// Устанавливает перезагружаемый фильтр для тестов без глобального подписчика.
///
/// Тесты не поднимают процессный `tracing`, но сценарии уровня журнала всё
/// равно проверяют успешное применение, поэтому фильтр инициализируется отдельно.
/// Слой хранится в `static`: ручка перезагрузки ссылается на него слабо, и
/// без хранения перезагрузка сообщала бы «subscriber no longer exists».
#[cfg(test)]
pub(crate) fn init_for_tests(filter: &str) {
    let (layer, reload_handle) = tracing_subscriber::reload::Layer::new(EnvFilter::new(filter));
    let _ = TEST_FILTER_LAYER.set(layer);
    let _ = STARTUP_FILTER.set(filter.to_owned());
    install_reload_filter::<tracing_subscriber::Registry>(reload_handle);
}

/// Слой фильтра, удерживаемый живым на время тестового процесса.
#[cfg(test)]
static TEST_FILTER_LAYER: OnceLock<
    tracing_subscriber::reload::Layer<EnvFilter, tracing_subscriber::Registry>,
> = OnceLock::new();

/// Сохраняет ручку перезагрузки для последующих вызовов [`set_log_level`].
///
/// Тип подписчика выводится в точке вызова: он собран из слоёв и не имеет
/// короткого имени, поэтому замыкание скрывает его от `static`.
fn install_reload_filter<S>(reload_handle: ReloadHandle<EnvFilter, S>)
where
    S: Subscriber + Send + Sync + 'static,
{
    let current_handle = reload_handle.clone();
    let _ = CURRENT_FILTER.set(Box::new(move || {
        current_handle
            .with_current(ToString::to_string)
            .map_err(|error| anyhow!("failed to read tracing filter: {error}"))
    }));
    let _ = RELOAD_FILTER.set(Box::new(move |directive: &str| {
        reload_handle
            .reload(EnvFilter::new(directive))
            .map_err(|error| anyhow!("failed to reload tracing filter: {error}"))
    }) as ReloadFilter);
}

/// Фильтр, заданный переменной окружения при запуске процесса.
static STARTUP_FILTER: OnceLock<String> = OnceLock::new();

/// Переустановить фильтр процесса по новой директиве.
///
/// Замыкание хранит ручку `tracing_subscriber::reload`, чей тип подписчика
/// собран из слоёв и не пригоден для явного именования в `static`.
type ReloadFilter = Box<dyn Fn(&str) -> anyhow::Result<()> + Send + Sync>;

/// Заполняется в [`init`] один раз за время работы процесса.
static RELOAD_FILTER: OnceLock<ReloadFilter> = OnceLock::new();

/// Применяет новый минимальный уровень журналирования ко всему процессу.
///
/// `None` возвращает фильтр, заданный при запуске. Вызов до [`init`] и ошибка
/// разбора директивы возвращаются вызывающему коду: молчаливая смена уровня
/// опаснее явного отказа.
pub(crate) fn set_log_level(level: Option<&str>) -> anyhow::Result<()> {
    let Some(reload_filter) = RELOAD_FILTER.get() else {
        anyhow::bail!("tracing filter is not initialized");
    };
    let directive = match level {
        Some(level) => level.to_owned(),
        None => STARTUP_FILTER
            .get()
            .cloned()
            .ok_or_else(|| anyhow!("startup tracing filter is unknown"))?,
    };
    reload_filter(&directive)
}

/// Возвращает директиву из живого слоя для изолированных проверок настроек.
#[cfg(test)]
pub(crate) fn current_filter_for_tests() -> String {
    TEST_FILTER_LAYER
        .get()
        .unwrap()
        .handle()
        .with_current(ToString::to_string)
        .unwrap()
}

/// Читает фактическую директиву слоя перед временным изменением.
static CURRENT_FILTER: OnceLock<Box<dyn Fn() -> anyhow::Result<String> + Send + Sync>> =
    OnceLock::new();

/// Откатывает временный фильтр, если связанное сохранение не завершилось.
///
/// Вызывающий код обязан сериализовать изменения фильтра до уничтожения
/// этой защиты, иначе откат мог бы затереть более новое изменение.
pub(crate) struct LogFilterChange {
    previous: Option<String>,
}

impl LogFilterChange {
    /// Оставляет новый фильтр после успешного сохранения настройки.
    pub(crate) fn commit(mut self) {
        self.previous = None;
    }
}

impl Drop for LogFilterChange {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take()
            && let Some(reload) = RELOAD_FILTER.get()
            && let Err(error) = reload(&previous)
        {
            tracing::error!(%error, "failed to roll back host log filter");
        }
    }
}

/// Применяет фильтр с синхронным откатом до подтверждения сохранения.
///
/// Возвращает ошибку чтения или применения до изменения настроек в хранилище.
/// Защита восстанавливает фактическую директиву, включая фильтры отдельных targets.
pub(crate) fn change_log_level(level: Option<&str>) -> anyhow::Result<LogFilterChange> {
    let current = CURRENT_FILTER
        .get()
        .ok_or_else(|| anyhow!("tracing filter is not initialized"))?;
    let previous = current()?;
    set_log_level(level)?;
    Ok(LogFilterChange {
        previous: Some(previous),
    })
}

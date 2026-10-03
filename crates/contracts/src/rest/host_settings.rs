//! REST-контракты глобальных настроек хоста CheenHub.

use serde::{Deserialize, Serialize};

/// Транспорт исходящих системных писем.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailTransport {
    /// Отправка через SMTP.
    Smtp,
    /// Отправка через Gmail API по HTTPS.
    GmailApi,
}

/// Результат проверки прав владельца хоста.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct HostAccessResponse {
    /// Имеет ли текущий пользователь права владельца этого хоста CheenHub.
    pub is_host_owner: bool,
}

/// История нагрузки хоста, доступная владельцу установки.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct HostMetricsResponse {
    /// Доступен ли источник актуальных системных метрик.
    pub available: bool,
    /// Последние измерения в хронологическом порядке.
    pub samples: Vec<HostMetricsSample>,
}

/// Текущая активность голосового чата на хосте.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct HostVoiceActivityResponse {
    /// Число активных голосовых подключений: комнаты и личные звонки вместе.
    pub voice_connections: u32,
    /// Число активных видеоисточников: камера и экран считаются отдельно.
    pub video_sources: u32,
}

/// История активности голосового чата, доступная владельцу установки.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct HostVoiceActivityHistoryResponse {
    /// Доступна ли история: при сбое базы данных текущая активность остаётся доступна.
    pub available: bool,
    /// Снимки в хронологическом порядке; пусто при недоступной истории.
    pub samples: Vec<HostVoiceActivitySample>,
}

/// Один снимок активности голосового чата на хосте.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct HostVoiceActivitySample {
    /// Время измерения в миллисекундах Unix.
    pub sampled_at_unix_ms: i64,
    /// Число активных голосовых подключений в момент измерения.
    pub voice_connections: u32,
    /// Число активных видеоисточников в момент измерения: камера и экран считаются раздельно.
    pub video_sources: u32,
}

/// Одно измерение нагрузки хоста.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct HostMetricsSample {
    /// Время измерения в миллисекундах Unix.
    pub sampled_at_unix_ms: i64,
    /// Использование процессора.
    pub cpu: HostCpuMetrics,
    /// Использование оперативной памяти.
    pub memory: HostMemoryMetrics,
    /// Использование накопителя хоста; отсутствует у старых metrics-proxy или при недоступности диска.
    #[serde(default)]
    pub disk: Option<HostDiskMetrics>,
    /// Сетевой трафик CheenHub.
    pub network: HostNetworkMetrics,
}

/// Использование процессора хостом и сервисами CheenHub.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct HostCpuMetrics {
    /// Общая занятость процессора хоста в процентах от полной мощности.
    pub system_percent: f32,
    /// Доля полной мощности процессора, занятая CheenHub.
    pub cheenhub_percent: f32,
    /// Доля полной мощности процессора, занятая базой данных CheenHub.
    pub database_percent: f32,
    /// Доля полной мощности процессора, занятая остальной системой.
    pub other_percent: f32,
    /// Занятость каждого логического процессора от 0 до 100 процентов.
    pub logical_processors_percent: Vec<f32>,
}

/// Использование оперативной памяти хостом и сервисами CheenHub.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct HostMemoryMetrics {
    /// Общий объём оперативной памяти хоста.
    pub total_bytes: u64,
    /// Используемый объём оперативной памяти хоста.
    pub used_bytes: u64,
    /// Память контейнеров CheenHub без базы данных.
    pub cheenhub_bytes: u64,
    /// Память контейнера базы данных CheenHub.
    pub database_bytes: u64,
    /// Память, занятая остальной системой.
    pub other_bytes: u64,
}

/// Использование дискового пространства накопителя хоста.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct HostDiskMetrics {
    /// Общий объём накопителя хоста в байтах.
    pub total_bytes: u64,
    /// Занятый объём накопителя хоста в байтах.
    pub used_bytes: u64,
}

/// Сетевой трафик только контейнеров CheenHub без базы данных.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct HostNetworkMetrics {
    /// Скорость исходящего трафика в байтах в секунду.
    pub sent_bytes_per_second: f64,
    /// Скорость входящего трафика в байтах в секунду.
    pub received_bytes_per_second: f64,
    /// Суммарный исходящий трафик с момента запуска контейнеров.
    pub sent_bytes_total: u64,
    /// Суммарный входящий трафик с момента запуска контейнеров.
    pub received_bytes_total: u64,
}

/// Настройки отправки почты без сохранённых секретов.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct HostEmailSettingsResponse {
    /// Активный транспорт отправки.
    pub transport: EmailTransport,
    /// Максимальное время одной операции отправки в секундах.
    pub email_send_timeout_seconds: u64,
    /// Адрес SMTP-сервера.
    pub smtp_host: Option<String>,
    /// Порт SMTP-сервера.
    pub smtp_port: u16,
    /// Имя пользователя SMTP.
    pub smtp_username: Option<String>,
    /// Настроен ли пароль SMTP.
    pub smtp_password_configured: bool,
    /// Адрес отправителя SMTP.
    pub smtp_from_email: Option<String>,
    /// ID OAuth-клиента Gmail.
    pub gmail_client_id: Option<String>,
    /// Взят ли ID OAuth-клиента из `GOOGLE_OAUTH_CLIENT_ID` вместо БД.
    pub gmail_client_id_from_environment: bool,
    /// Настроен ли секрет OAuth-клиента Gmail.
    pub gmail_client_secret_configured: bool,
    /// Взят ли секрет OAuth-клиента из `GOOGLE_OAUTH_CLIENT_SECRET` вместо БД.
    pub gmail_client_secret_from_environment: bool,
    /// Подключён ли Gmail и сохранён ли refresh token.
    pub gmail_connected: bool,
    /// Подтверждённый адрес подключённого Gmail-аккаунта.
    pub gmail_from_email: Option<String>,
    /// Точный callback URI, который нужно зарегистрировать в Google Cloud.
    pub gmail_oauth_redirect_uri: String,
}

/// Изменяемые настройки отправки почты владельцем хоста.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct UpdateHostEmailSettingsRequest {
    /// Новый активный транспорт, если его требуется изменить.
    pub transport: Option<EmailTransport>,
    /// Новый таймаут отправки от 1 до 300 секунд.
    pub email_send_timeout_seconds: Option<u64>,
    /// Новый адрес SMTP-сервера; пустая строка очищает поле.
    pub smtp_host: Option<String>,
    /// Новый порт SMTP-сервера.
    pub smtp_port: Option<u16>,
    /// Новое имя пользователя SMTP; пустая строка очищает поле.
    pub smtp_username: Option<String>,
    /// Новый пароль SMTP; пустая строка сохраняет прежний пароль.
    pub smtp_password: Option<String>,
    /// Явно удалить сохранённый пароль SMTP.
    pub clear_smtp_password: Option<bool>,
    /// Новый адрес отправителя SMTP; пустая строка очищает поле.
    pub smtp_from_email: Option<String>,
    /// Новый ID OAuth-клиента Gmail; пустая строка очищает поле.
    pub gmail_client_id: Option<String>,
    /// Новый секрет OAuth-клиента Gmail; пустая строка сохраняет прежний секрет.
    pub gmail_client_secret: Option<String>,
    /// Явно удалить сохранённый секрет OAuth-клиента Gmail.
    pub clear_gmail_client_secret: Option<bool>,
}

/// Результат запуска подключения Gmail.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GmailConnectionStartResponse {
    /// URL Google OAuth, на который следует перейти браузеру.
    pub authorization_url: String,
}

/// Одна запись оперативного журнала бэкенда.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct HostLogEntry {
    /// Монотонный идентификатор записи в рамках текущего процесса.
    pub id: u64,
    /// Время события в RFC 3339.
    pub timestamp: String,
    /// Уровень `tracing`.
    pub level: String,
    /// Target события `tracing`.
    pub target: String,
    /// Основное сообщение события.
    pub message: String,
    /// Дополнительные структурированные поля в безопасном текстовом виде.
    pub fields: Vec<String>,
}

/// Сообщение realtime-потока журнала бэкенда.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostLogStreamMessage {
    /// Снимок последних записей при подключении или восстановлении после отставания.
    Snapshot {
        /// Записи в хронологическом порядке.
        entries: Vec<HostLogEntry>,
    },
    /// Новая запись журнала.
    Entry {
        /// Новая запись.
        entry: HostLogEntry,
    },
    /// Ошибка доступа или работы потока.
    Error {
        /// Безопасное сообщение для интерфейса.
        message: String,
        /// Можно ли автоматически переподключиться.
        retryable: bool,
    },
}

#[cfg(test)]
mod tests;

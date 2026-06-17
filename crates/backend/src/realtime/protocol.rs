//! Вспомогательные функции протокола конвертов realtime.

use anyhow::{Context, anyhow};
use cheenhub_contracts::realtime::{
    ControlKind, RealtimeEnvelope, RealtimeKind, RealtimeModule, Rejected, RejectionCode,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use super::sink::EnvelopeSink;

/// Убеждается, что конверт имеет соответствующую пару модуль/вид.
pub(crate) fn validate_envelope(envelope: &RealtimeEnvelope) -> anyhow::Result<()> {
    if envelope.has_matching_module_kind() {
        Ok(())
    } else {
        Err(anyhow!("realtime module and kind do not match"))
    }
}

/// Возвращает идентификатор запроса или завершается с ошибкой, если он отсутствует в сообщении запрос-ответ.
pub(crate) fn require_request_id(envelope: &RealtimeEnvelope) -> anyhow::Result<Uuid> {
    envelope
        .request_id
        .ok_or_else(|| anyhow!("realtime request is missing request_id"))
}

/// Декодирует типизированную полезную нагрузку из конверта.
pub(crate) fn decode_payload<T>(envelope: &RealtimeEnvelope) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(envelope.payload.clone()).context("failed to decode realtime payload")
}

/// Отправляет типизированный конверт с отказом.
pub(crate) async fn send_rejection(
    send: &EnvelopeSink,
    request_id: Option<Uuid>,
    code: RejectionCode,
    message: &str,
) -> anyhow::Result<()> {
    write_envelope(
        send,
        RealtimeModule::Control,
        RealtimeKind::Control(ControlKind::Rejected),
        request_id,
        Rejected {
            code,
            message: message.to_owned(),
        },
    )
    .await
}

/// Записывает типизированную полезную нагрузку как realtime-конверт.
pub(crate) async fn write_envelope<T>(
    send: &EnvelopeSink,
    module: RealtimeModule,
    kind: RealtimeKind,
    request_id: Option<Uuid>,
    payload: T,
) -> anyhow::Result<()>
where
    T: Serialize,
{
    let envelope = RealtimeEnvelope::new(module, kind, request_id, payload)?;
    send.send_envelope(&envelope).await
}

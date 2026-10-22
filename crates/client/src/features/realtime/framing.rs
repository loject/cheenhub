//! Фрейминг realtime-потока с префиксом длины.

use bytes::{BufMut, Bytes, BytesMut};
use cheenhub_contracts::realtime::{RealtimeEnvelope, RealtimeKind, RealtimeModule};
use dioxus::prelude::warn;
use futures_util::lock::Mutex;
use std::rc::Rc;
use uuid::Uuid;
use web_transport::{RecvStream, SendStream};

use super::error::RealtimeError;

const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

#[cfg(test)]
mod tests;

#[derive(serde::Deserialize)]
struct RawRealtimeEnvelope {
    module: String,
    kind: String,
    request_id: Option<Uuid>,
    payload: serde_json::Value,
}

/// Результат чтения конверта: известное сообщение или неизвестное событие.
pub(crate) enum InboundRealtimeEnvelope {
    /// Известный модуль и тип сообщения.
    Known(RealtimeEnvelope),
    /// Неизвестный модуль или тип; событие можно безопасно пропустить.
    Unknown { module: String, kind: String },
}

/// Разбирает JSON, отличая новые типы событий от поврежденного конверта.
pub(crate) fn decode_inbound_envelope(
    bytes: &[u8],
) -> Result<InboundRealtimeEnvelope, RealtimeError> {
    let raw: RawRealtimeEnvelope = serde_json::from_slice(bytes).map_err(|error| {
        RealtimeError::new(format!("Failed to decode realtime envelope: {error}"))
    })?;
    let module =
        serde_json::from_value::<RealtimeModule>(serde_json::Value::String(raw.module.clone()));
    let kind = serde_json::from_value::<RealtimeKind>(serde_json::Value::String(raw.kind.clone()));

    match (module, kind) {
        (Ok(module), Ok(kind)) => Ok(InboundRealtimeEnvelope::Known(RealtimeEnvelope {
            module,
            kind,
            request_id: raw.request_id,
            payload: raw.payload,
        })),
        _ => Ok(InboundRealtimeEnvelope::Unknown {
            module: raw.module,
            kind: raw.kind,
        }),
    }
}

/// Записывает один JSON-фрейм конверта в надежный поток.
pub(crate) async fn write_envelope(
    send: &Rc<Mutex<SendStream>>,
    envelope: &RealtimeEnvelope,
) -> Result<(), RealtimeError> {
    let bytes = serde_json::to_vec(envelope).map_err(|error| {
        RealtimeError::new(format!("Failed to encode realtime envelope: {error}"))
    })?;
    write_frame(send, &bytes).await
}

/// Читает следующий известный конверт, пропуская неизвестные события.
pub(crate) async fn read_envelope(
    recv: &mut RecvStream,
) -> Result<Option<RealtimeEnvelope>, RealtimeError> {
    loop {
        let Some(frame) = read_frame(recv).await? else {
            return Ok(None);
        };

        match decode_inbound_envelope(&frame)? {
            InboundRealtimeEnvelope::Known(envelope) => return Ok(Some(envelope)),
            InboundRealtimeEnvelope::Unknown { module, kind } => {
                warn!(%module, %kind, "ignoring unknown realtime event");
            }
        }
    }
}

async fn write_frame(send: &Rc<Mutex<SendStream>>, payload: &[u8]) -> Result<(), RealtimeError> {
    if payload.len() > MAX_FRAME_BYTES {
        return Err(RealtimeError::new("Realtime frame is too large."));
    }

    let mut frame = BytesMut::with_capacity(4 + payload.len());
    frame.put_u32(payload.len() as u32);
    frame.extend_from_slice(payload);

    let mut written = 0;
    let mut send = send.lock().await;
    while written < frame.len() {
        let count = send.write(&frame[written..]).await.map_err(|error| {
            RealtimeError::new(format!("Failed to write realtime frame: {error}"))
        })?;
        if count == 0 {
            return Err(RealtimeError::new("Realtime stream refused frame bytes."));
        }
        written += count;
    }

    Ok(())
}

async fn read_frame(recv: &mut RecvStream) -> Result<Option<Bytes>, RealtimeError> {
    let Some(length_bytes) = read_exact(recv, 4).await? else {
        return Ok(None);
    };
    let length = u32::from_be_bytes(length_bytes[..4].try_into().expect("four bytes")) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(RealtimeError::new("Realtime frame exceeds max size."));
    }

    read_exact(recv, length).await
}

async fn read_exact(recv: &mut RecvStream, length: usize) -> Result<Option<Bytes>, RealtimeError> {
    let mut buffer = BytesMut::with_capacity(length);
    while buffer.len() < length {
        let remaining = length - buffer.len();
        let Some(chunk) = recv.read(remaining).await.map_err(|error| {
            RealtimeError::new(format!("Failed to read realtime frame: {error}"))
        })?
        else {
            if buffer.is_empty() {
                return Ok(None);
            }
            return Err(RealtimeError::new("Realtime stream closed mid-frame."));
        };
        buffer.extend_from_slice(&chunk);
    }

    Ok(Some(buffer.freeze()))
}

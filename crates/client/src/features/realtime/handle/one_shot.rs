//! Выполнение отменяемого одноразового realtime-запроса через WebTransport.

use std::rc::Rc;

use cheenhub_contracts::realtime::{ControlKind, RealtimeEnvelope, RealtimeKind, RealtimeModule};
use dioxus::prelude::{debug, warn};
use futures_util::lock::Mutex;
use uuid::Uuid;
use web_transport::Session;

use crate::features::realtime::RealtimeError;
use crate::features::realtime::framing;

/// Записывает запрос и читает его единственный ответ в рамках одного future.
///
/// `send` и `recv` остаются локальными владельцами future: его отмена уничтожает оба потока и не
/// оставляет отдельную задачу чтения или запись в pending map.
pub(super) async fn request(
    envelope: RealtimeEnvelope,
    session: Session,
    request_id: Uuid,
) -> Result<RealtimeEnvelope, RealtimeError> {
    let module = envelope.module;
    let (send, mut recv) = session
        .open_bi()
        .await
        .map_err(|error| RealtimeError::new(format!("Failed to open realtime stream: {error}")))?;
    let send = Rc::new(Mutex::new(send));
    debug!(?module, %request_id, "opened one-shot WebTransport realtime stream");

    if let Err(error) = framing::write_envelope(&send, &envelope).await {
        send.lock().await.reset(0);
        warn!(?module, %request_id, %error, "one-shot frame write failed; request not replayed");
        return Err(error);
    }
    debug!(?module, %request_id, "wrote one-shot realtime request");
    let response = framing::read_envelope(&mut recv).await?.ok_or_else(|| {
        RealtimeError::new("Realtime stream closed before the request completed.")
    })?;
    validate_response(module, request_id, &response)?;
    Ok(response)
}

fn validate_response(
    module: RealtimeModule,
    request_id: Uuid,
    response: &RealtimeEnvelope,
) -> Result<(), RealtimeError> {
    let is_rejection = response.kind == RealtimeKind::Control(ControlKind::Rejected);
    if !response.has_matching_module_kind()
        || (!is_rejection && response.module != module)
        || response.request_id != Some(request_id)
    {
        return Err(RealtimeError::new(
            "Realtime one-shot stream returned a mismatched response.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;

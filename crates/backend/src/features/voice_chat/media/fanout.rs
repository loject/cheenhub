//! Совместимая ретрансляция v2/v3 с серверной идентичностью голосовой сессии.

use bytes::{Bytes, BytesMut};
use cheenhub_contracts::media::{MediaDatagramError, MediaDatagramHeader};
use tracing::warn;
use uuid::Uuid;

use crate::state::AppState;

/// Добавляет подтверждённый сервером UUID голосовой сессии к v2 для новых получателей.
/// Исходный v2 не меняется и может быть передан старым клиентам без копирования.
pub(super) fn stamp_relay_voice_session(
    bytes: &Bytes,
    session_id: Uuid,
) -> Result<Bytes, MediaDatagramError> {
    let header = MediaDatagramHeader::decode(bytes)?;
    if bytes[4] != 2 || session_id.is_nil() {
        return Err(MediaDatagramError::InvalidTrack);
    }
    let mut stamped = BytesMut::with_capacity(header.wire_len() + 16);
    stamped.extend_from_slice(&bytes[..82]);
    stamped[4] = 3;
    stamped.extend_from_slice(session_id.as_bytes());
    stamped.extend_from_slice(header.payload(bytes)?);
    Ok(stamped.freeze())
}

/// Ретранслирует готовую датаграмму v2/v3 одновременно двум поколениям клиентов.
pub(super) async fn fanout_relay_bytes(
    state: &AppState,
    legacy_recipients: &[Uuid],
    modern_recipients: &[Uuid],
    sender_session_id: Uuid,
    bytes: Bytes,
) {
    let has_legacy = legacy_recipients.iter().any(|id| *id != sender_session_id);
    let has_modern = modern_recipients.iter().any(|id| *id != sender_session_id);
    let send_legacy = async {
        if has_legacy {
            state
                .realtime_hub
                .fanout_datagram_to_sessions_except(
                    legacy_recipients,
                    sender_session_id,
                    bytes.clone(),
                )
                .await;
        }
    };
    let send_modern = async {
        if has_modern {
            match stamp_relay_voice_session(&bytes, sender_session_id) {
                Ok(stamped) => {
                    state
                        .realtime_hub
                        .fanout_datagram_to_sessions_except(
                            modern_recipients,
                            sender_session_id,
                            stamped,
                        )
                        .await;
                }
                Err(error) => {
                    warn!(%sender_session_id, %error, "failed to stamp authenticated voice session on media relay");
                }
            }
        }
    };
    tokio::join!(send_legacy, send_modern);
}

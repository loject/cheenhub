//! Модуль управления realtime.

use cheenhub_contracts::realtime::{
    Authenticate, Authenticated, ControlAck, ControlKind, ControlText, RealtimeEnvelope,
    RealtimeKind, RealtimeModule, RejectionCode,
};
use cheenhub_contracts::rest::AuthUser;
use tracing::{info, warn};

use crate::features::auth::application as auth_application;
use crate::state::AppState;

use super::protocol::{
    decode_payload, require_request_id, send_rejection, validate_envelope, write_envelope,
};
use super::sink::EnvelopeSink;

/// Результат проверки первого сообщения realtime-сессии.
pub(crate) struct AuthenticatedRealtimeSession {
    /// Пользователь, доступный обработчикам продуктовых realtime-модулей.
    pub(crate) user: AuthUser,
    /// Auth-сессия, отзыв которой должен завершить этот realtime-транспорт.
    pub(crate) auth_session_id: uuid::Uuid,
}

/// Аутентифицирует первый поток realtime-сессии.
pub(crate) async fn authenticate_session(
    state: &AppState,
    send: &EnvelopeSink,
    envelope: RealtimeEnvelope,
) -> anyhow::Result<Option<AuthenticatedRealtimeSession>> {
    validate_envelope(&envelope)?;

    if envelope.module != RealtimeModule::Control
        || envelope.kind != RealtimeKind::Control(ControlKind::Authenticate)
    {
        send_rejection(
            send,
            envelope.request_id,
            RejectionCode::Unauthorized,
            "Первое realtime сообщение должно авторизовать сессию.",
        )
        .await?;
        return Ok(None);
    }

    let request_id = require_request_id(&envelope)?;
    let auth: Authenticate = decode_payload(&envelope)?;
    let (user_account, auth_session_id) =
        match auth_application::require_current_user(state, &auth.access_token).await {
            Ok(authenticated) => authenticated,
            Err(error) => {
                warn!(?error, "rejected realtime authentication");
                send_rejection(
                    send,
                    Some(request_id),
                    RejectionCode::Unauthorized,
                    "Сессия истекла. Войди снова.",
                )
                .await?;
                return Ok(None);
            }
        };
    let user = auth_application::auth_user(state, &user_account);
    let user_id = user.id.clone();

    write_envelope(
        send,
        RealtimeModule::Control,
        RealtimeKind::Control(ControlKind::Authenticated),
        Some(request_id),
        Authenticated { user: user.clone() },
    )
    .await?;

    info!(%user_id, "accepted realtime authentication");

    Ok(Some(AuthenticatedRealtimeSession {
        user,
        auth_session_id,
    }))
}

/// Обрабатывает один конверт модуля управления.
pub(crate) async fn handle(
    _state: &AppState,
    send: &EnvelopeSink,
    envelope: RealtimeEnvelope,
) -> anyhow::Result<()> {
    match envelope.kind {
        RealtimeKind::Control(ControlKind::ControlText) => {
            let request_id = require_request_id(&envelope)?;
            let payload: ControlText = decode_payload(&envelope)?;
            write_envelope(
                send,
                RealtimeModule::Control,
                RealtimeKind::Control(ControlKind::ControlAck),
                Some(request_id),
                ControlAck {
                    body: format!("received: {}", payload.body),
                },
            )
            .await
        }
        RealtimeKind::Control(ControlKind::Authenticate) => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::BadRequest,
                "Realtime session is already authenticated.",
            )
            .await
        }
        RealtimeKind::Control(_) => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::UnsupportedMessage,
                "Unsupported control message.",
            )
            .await
        }
        _ => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::BadRequest,
                "Realtime kind does not belong to control module.",
            )
            .await
        }
    }
}

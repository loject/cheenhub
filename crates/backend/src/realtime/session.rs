//! WebTransport realtime session lifecycle.

use anyhow::{Context, anyhow};
use std::sync::Arc;

use cheenhub_contracts::realtime::RealtimeModule;
use cheenhub_contracts::rest::AuthUser;
use tokio::sync::Mutex;
use tracing::{debug, info};
use uuid::Uuid;
use web_transport::{RecvStream, SendStream, Session};

use crate::state::AppState;

use super::framing;
use super::protocol::validate_envelope;
use super::{control, router};

/// Handles one accepted WebTransport session.
pub(crate) async fn handle_session(
    state: AppState,
    session_id: Uuid,
    session: Session,
) -> anyhow::Result<()> {
    info!(%session_id, "waiting for realtime authentication stream");
    let (send, recv) = session
        .accept_bi()
        .await
        .context("failed to accept auth stream")?;
    let send = Arc::new(Mutex::new(send));
    let mut recv = recv;

    let envelope = framing::read_envelope(&mut recv)
        .await?
        .ok_or_else(|| anyhow!("auth stream closed before authentication"))?;
    let Some(user) = control::authenticate_session(&state, &send, envelope).await? else {
        info!(%session_id, "closing unauthorized realtime session");
        session.close(4001, "unauthorized");
        return Ok(());
    };
    let user_id = Uuid::parse_str(&user.id).context("authenticated user id is not a uuid")?;
    info!(%session_id, %user_id, "authenticated realtime session");

    let state_for_control = state.clone();
    let user_for_control = user.clone();
    tokio::spawn(async move {
        if let Err(error) = handle_module_stream(
            ModuleStreamContext {
                state: state_for_control,
                user: user_for_control,
                user_id,
                session_id,
                stream_kind: "control",
            },
            send,
            recv,
            Some(RealtimeModule::Control),
        )
        .await
        {
            debug!(%session_id, %error, "control realtime stream closed");
        }
    });

    loop {
        let (send, recv) = match session.accept_bi().await {
            Ok(streams) => streams,
            Err(error) => {
                info!(
                    %session_id,
                    %user_id,
                    %error,
                    "realtime session closed while waiting for module stream"
                );
                return Ok(());
            }
        };
        debug!(%session_id, "accepted realtime module stream");
        let state = state.clone();
        let user = user.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_module_stream(
                ModuleStreamContext {
                    state,
                    user,
                    user_id,
                    session_id,
                    stream_kind: "module",
                },
                Arc::new(Mutex::new(send)),
                recv,
                None,
            )
            .await
            {
                debug!(%session_id, %error, "module realtime stream closed");
            }
        });
    }
}

struct ModuleStreamContext {
    state: AppState,
    user: AuthUser,
    user_id: Uuid,
    session_id: Uuid,
    stream_kind: &'static str,
}

async fn handle_module_stream(
    context: ModuleStreamContext,
    send: Arc<Mutex<SendStream>>,
    mut recv: RecvStream,
    mut stream_module: Option<RealtimeModule>,
) -> anyhow::Result<()> {
    let stream_id = Uuid::new_v4();
    let mut registered_stream = false;
    let result = async {
        while let Some(envelope) = framing::read_envelope(&mut recv).await? {
            validate_envelope(&envelope)?;

            match stream_module {
                Some(module) if module != envelope.module => {
                    router::reject_module_change(&send, &envelope).await?;
                    return Ok(());
                }
                Some(_) => {}
                None => {
                    debug!(
                        session_id = %context.session_id,
                        stream_kind = context.stream_kind,
                        module = ?envelope.module,
                        "bound realtime stream to module"
                    );
                    stream_module = Some(envelope.module);
                    context
                        .state
                        .realtime_hub
                        .register_stream(stream_id, envelope.module, context.user_id, send.clone())
                        .await;
                    registered_stream = true;
                }
            }

            router::dispatch(
                &context.state,
                &context.user,
                &context.user_id,
                stream_id,
                &send,
                envelope,
            )
            .await?;
        }

        Ok(())
    }
    .await;

    if registered_stream {
        context
            .state
            .realtime_hub
            .unregister_stream(stream_id)
            .await;
        if let Some(module) = stream_module {
            router::cleanup_stream(&context.state, module, stream_id).await;
        }
    }

    result
}

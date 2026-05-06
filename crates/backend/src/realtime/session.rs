//! WebTransport realtime session lifecycle.

use anyhow::{Context, anyhow};
use cheenhub_contracts::realtime::RealtimeModule;
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
    let send = Mutex::new(send);
    let mut recv = recv;

    let envelope = framing::read_envelope(&mut recv)
        .await?
        .ok_or_else(|| anyhow!("auth stream closed before authentication"))?;
    let Some(user_id) = control::authenticate_session(&state, &send, envelope).await? else {
        info!(%session_id, "closing unauthorized realtime session");
        session.close(4001, "unauthorized");
        return Ok(());
    };
    info!(%session_id, %user_id, "authenticated realtime session");

    let state_for_control = state.clone();
    tokio::spawn(async move {
        if let Err(error) =
            handle_module_stream(state_for_control, session_id, "control", send, recv, None).await
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
        tokio::spawn(async move {
            if let Err(error) =
                handle_module_stream(state, session_id, "module", Mutex::new(send), recv, None)
                    .await
            {
                debug!(%session_id, %error, "module realtime stream closed");
            }
        });
    }
}

async fn handle_module_stream(
    _state: AppState,
    session_id: Uuid,
    stream_kind: &'static str,
    send: Mutex<SendStream>,
    mut recv: RecvStream,
    mut stream_module: Option<RealtimeModule>,
) -> anyhow::Result<()> {
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
                    %session_id,
                    stream_kind,
                    module = ?envelope.module,
                    "bound realtime stream to module"
                );
                stream_module = Some(envelope.module);
            }
        }

        router::dispatch(&_state, &send, envelope).await?;
    }

    Ok(())
}

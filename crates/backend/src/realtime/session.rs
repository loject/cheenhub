//! WebTransport realtime session lifecycle.

use anyhow::{Context, anyhow};
use cheenhub_contracts::realtime::RealtimeModule;
use tokio::sync::Mutex;
use tracing::debug;
use web_transport::{RecvStream, SendStream, Session};

use crate::state::AppState;

use super::framing;
use super::protocol::validate_envelope;
use super::{control, router};

/// Handles one accepted WebTransport session.
pub(crate) async fn handle_session(state: AppState, session: Session) -> anyhow::Result<()> {
    let (send, recv) = session
        .accept_bi()
        .await
        .context("failed to accept auth stream")?;
    let send = Mutex::new(send);
    let mut recv = recv;

    let envelope = framing::read_envelope(&mut recv)
        .await?
        .ok_or_else(|| anyhow!("auth stream closed before authentication"))?;
    if !control::authenticate_session(&state, &send, envelope).await? {
        session.close(4001, "unauthorized");
        return Ok(());
    }

    let state_for_control = state.clone();
    tokio::spawn(async move {
        if let Err(error) = handle_module_stream(state_for_control, send, recv, None).await {
            debug!(%error, "control realtime stream closed");
        }
    });

    loop {
        let (send, recv) = session
            .accept_bi()
            .await
            .context("failed to accept module stream")?;
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_module_stream(state, Mutex::new(send), recv, None).await {
                debug!(%error, "module realtime stream closed");
            }
        });
    }
}

async fn handle_module_stream(
    _state: AppState,
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
            None => stream_module = Some(envelope.module),
        }

        router::dispatch(&_state, &send, envelope).await?;
    }

    Ok(())
}

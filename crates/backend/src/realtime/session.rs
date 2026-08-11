//! Жизненный цикл realtime-сессии WebTransport.

use anyhow::{Context, anyhow};
use std::{sync::Arc, time::Duration};

use cheenhub_contracts::realtime::RealtimeModule;
use cheenhub_contracts::rest::AuthUser;
use tokio::{
    sync::{Mutex, Semaphore},
    time::timeout,
};
use tracing::{debug, info, warn};
use uuid::Uuid;
use web_transport::{RecvStream, Session};

use crate::features::auth::application as auth_application;
use crate::state::AppState;

use super::framing;
use super::protocol::validate_envelope;
use super::sink::{DatagramSink, EnvelopeSink};
use super::{control, datagram, router};

const AUTHENTICATION_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_CONCURRENT_MODULE_STREAMS: usize = 32;

/// Обрабатывает одну принятую сессию WebTransport.
pub(crate) async fn handle_session(
    state: AppState,
    session_id: Uuid,
    session: Session,
) -> anyhow::Result<()> {
    info!(%session_id, "waiting for realtime authentication stream");
    let authentication = async {
        let (send, recv) = session
            .accept_bi()
            .await
            .context("failed to accept auth stream")?;
        let send = EnvelopeSink::webtransport(Arc::new(Mutex::new(send)));
        let mut recv = recv;
        let envelope = framing::read_envelope(&mut recv)
            .await?
            .ok_or_else(|| anyhow!("auth stream closed before authentication"))?;
        let user = control::authenticate_session(&state, &send, envelope).await?;
        Ok::<_, anyhow::Error>((send, recv, user))
    };
    let (send, recv, user) = match timeout(AUTHENTICATION_TIMEOUT, authentication).await {
        Ok(result) => result?,
        Err(_) => {
            warn!(
                %session_id,
                timeout_seconds = AUTHENTICATION_TIMEOUT.as_secs(),
                "истёк таймаут первичной аутентификации realtime-сессии"
            );
            session.close(4008, "authentication timeout");
            return Ok(());
        }
    };
    let Some(authenticated) = user else {
        info!(%session_id, "closing unauthorized realtime session");
        session.close(4001, "unauthorized");
        return Ok(());
    };
    let user = authenticated.user;
    let auth_session_id = authenticated.auth_session_id;
    let user_id = Uuid::parse_str(&user.id).context("authenticated user id is not a uuid")?;
    info!(%session_id, %user_id, %auth_session_id, "authenticated realtime session");
    let mut disconnect = state
        .realtime_hub
        .register_session(
            session_id,
            user_id,
            auth_session_id,
            DatagramSink::webtransport(session.clone()),
        )
        .await;
    let auth_session_is_active =
        match auth_application::auth_session_is_active(&state, &auth_session_id).await {
            Ok(active) => active,
            Err(error) => {
                warn!(
                    %session_id,
                    %user_id,
                    %auth_session_id,
                    %error,
                    "failed to revalidate auth session after realtime registration"
                );
                state.realtime_hub.unregister_session(session_id).await;
                session.close(1011, "authentication unavailable");
                return Err(error);
            }
        };
    if !auth_session_is_active {
        warn!(
            %session_id,
            %user_id,
            %auth_session_id,
            "closing realtime transport whose auth session was revoked during registration"
        );
        state
            .realtime_hub
            .disconnect_auth_session(&auth_session_id)
            .await;
        session.close(4003, "auth session revoked");
        return Ok(());
    }
    datagram::spawn_reader(state.clone(), session_id, user_id, session.clone());

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

    let module_stream_slots = Arc::new(Semaphore::new(MAX_CONCURRENT_MODULE_STREAMS));
    let mut module_stream_limit_reported = false;
    loop {
        if !module_stream_limit_reported && module_stream_slots.available_permits() == 0 {
            warn!(
                %session_id,
                %user_id,
                max_concurrent_streams = MAX_CONCURRENT_MODULE_STREAMS,
                "достигнут лимит параллельных module streams realtime-сессии"
            );
            module_stream_limit_reported = true;
        }
        let accept_module_stream = async {
            let slot = module_stream_slots
                .clone()
                .acquire_owned()
                .await
                .context("realtime module stream semaphore is closed")?;
            let streams = session.accept_bi().await;
            Ok::<_, anyhow::Error>((slot, streams))
        };
        let (module_stream_slot, accepted) = tokio::select! {
            biased;
            _ = disconnect.changed() => {
                info!(
                    %session_id,
                    %user_id,
                    %auth_session_id,
                    "closing realtime transport after auth session revocation"
                );
                session.close(4003, "auth session revoked");
                state.realtime_hub.unregister_session(session_id).await;
                return Ok(());
            }
            result = accept_module_stream => result?,
        };
        let (send, recv) = match accepted {
            Ok(streams) => streams,
            Err(error) => {
                info!(
                    %session_id,
                    %user_id,
                    %error,
                    "realtime session closed while waiting for module stream"
                );
                state.realtime_hub.unregister_session(session_id).await;
                return Ok(());
            }
        };
        debug!(%session_id, "accepted realtime module stream");
        let state = state.clone();
        let user = user.clone();
        tokio::spawn(async move {
            let _module_stream_slot = module_stream_slot;
            if let Err(error) = handle_module_stream(
                ModuleStreamContext {
                    state,
                    user,
                    user_id,
                    session_id,
                    stream_kind: "module",
                },
                EnvelopeSink::webtransport(Arc::new(Mutex::new(send))),
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
    send: EnvelopeSink,
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
                context.session_id,
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

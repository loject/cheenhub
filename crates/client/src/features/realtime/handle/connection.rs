//! Подключение realtime transports.

use std::rc::Rc;

use cheenhub_contracts::realtime::{
    Authenticate, Authenticated, ControlKind, RealtimeKind, RealtimeModule,
};
use dioxus::prelude::{info, warn};
use futures_channel::mpsc;
use futures_util::FutureExt;
use futures_util::future::{Either, select};
use url::Url;
use web_time::Instant;
use web_transport::Session;

use super::{ConnectedSession, ConnectedTransport, RealtimeHandle};
use crate::features::realtime::config;
use crate::features::realtime::error::RealtimeError;
use crate::features::realtime::status::{
    RealtimeConnectionStatus, RealtimeFallbackInfo, RealtimeTransportKind,
    WebTransportFallbackReason,
};
use crate::features::realtime::{platform, websocket, webtransport};
use crate::features::runtime::sleep_ms;

const WEBTRANSPORT_CONNECT_TIMEOUT_MS: u32 = 1_500;
const WEBTRANSPORT_AUTH_TIMEOUT_MS: u32 = 10_000;

struct OpenWebTransport {
    url: Url,
    session: Session,
    generation: u64,
}

struct WebTransportFailure {
    info: RealtimeFallbackInfo,
    error: RealtimeError,
}

impl RealtimeHandle {
    /// Opens and authenticates the realtime session.
    pub(crate) async fn connect(
        &self,
        access_token: String,
    ) -> Result<Authenticated, RealtimeError> {
        let started_at = Instant::now();
        let webtransport = self.open_webtransport().boxed_local();
        let timeout = sleep_ms(WEBTRANSPORT_CONNECT_TIMEOUT_MS).boxed_local();
        let open_result = match select(webtransport, timeout).await {
            Either::Left((result, _)) => result.map_err(|error| WebTransportFailure {
                info: RealtimeFallbackInfo {
                    reason: classify_transport_failure(&error),
                    webtransport_elapsed_ms: elapsed_ms(started_at),
                },
                error,
            }),
            Either::Right(((), _)) => Err(WebTransportFailure {
                info: RealtimeFallbackInfo {
                    reason: WebTransportFallbackReason::Timeout,
                    webtransport_elapsed_ms: elapsed_ms(started_at),
                },
                error: RealtimeError::new(format!(
                    "WebTransport transport connection timed out after {WEBTRANSPORT_CONNECT_TIMEOUT_MS} ms"
                )),
            }),
        };

        let webtransport_result = match open_result {
            Ok(open) => {
                self.authenticate_webtransport(open, access_token.clone(), started_at)
                    .await
            }
            Err(failure) => Err(failure),
        };

        match webtransport_result {
            Ok(authenticated) => Ok(authenticated),
            Err(failure) => {
                warn!(
                    webtransport_error = %failure.error,
                    reason = ?failure.info.reason,
                    webtransport_elapsed_ms = failure.info.webtransport_elapsed_ms,
                    "WebTransport realtime connection failed; trying WebSocket fallback"
                );
                self.mark_connecting(RealtimeTransportKind::WebSocketFallback);
                self.connect_websocket(access_token, failure.info)
                    .await
                    .map_err(|websocket_error| {
                        RealtimeError::new(format!(
                            "Failed to connect realtime session: WebTransport error: {}; WebSocket fallback error: {websocket_error}",
                            failure.error
                        ))
                    })
            }
        }
    }

    async fn open_webtransport(&self) -> Result<OpenWebTransport, RealtimeError> {
        let url = config::realtime_url()?;
        info!(%url, "connecting WebTransport realtime session");
        let session = platform::connect(url.clone()).await?;

        info!(%url, "WebTransport transport connected");
        let generation = self.next_generation();
        self.inner.streams.lock().await.clear();
        self.inner.pending.borrow_mut().clear();
        self.inner.session.lock().await.replace(ConnectedSession {
            generation,
            transport: ConnectedTransport::WebTransport(Rc::new(session.clone())),
        });

        Ok(OpenWebTransport {
            url,
            session,
            generation,
        })
    }

    async fn authenticate_webtransport(
        &self,
        open: OpenWebTransport,
        access_token: String,
        started_at: Instant,
    ) -> Result<Authenticated, WebTransportFailure> {
        let authentication = self
            .request(
                RealtimeModule::Control,
                RealtimeKind::Control(ControlKind::Authenticate),
                Authenticate { access_token },
            )
            .boxed_local();
        let timeout = sleep_ms(WEBTRANSPORT_AUTH_TIMEOUT_MS).boxed_local();
        let authenticated: Result<Authenticated, RealtimeError> = match select(
            authentication,
            timeout,
        )
        .await
        {
            Either::Left((result, _)) => result,
            Either::Right(((), _)) => Err(RealtimeError::new(format!(
                "WebTransport realtime authentication timed out after {WEBTRANSPORT_AUTH_TIMEOUT_MS} ms"
            ))),
        };
        let authenticated = match authenticated {
            Ok(authenticated) => authenticated,
            Err(error) => {
                self.clear_generation(open.generation).await;
                return Err(WebTransportFailure {
                    info: RealtimeFallbackInfo {
                        reason: WebTransportFallbackReason::Authentication,
                        webtransport_elapsed_ms: elapsed_ms(started_at),
                    },
                    error,
                });
            }
        };
        info!(url = %open.url, user_id = %authenticated.user.id, "WebTransport realtime authenticated");
        self.set_connection_status(RealtimeConnectionStatus::Connected(
            RealtimeTransportKind::WebTransport,
        ));
        self.publish_fallback_info(None);
        webtransport::spawn_datagram_reader(
            open.session.clone(),
            open.generation,
            self.inner.datagram_listeners.clone(),
        );
        webtransport::spawn_connection_watcher(
            open.url.to_string(),
            open.session,
            open.generation,
            self.clone(),
        );

        Ok(authenticated)
    }

    async fn connect_websocket(
        &self,
        access_token: String,
        fallback_info: RealtimeFallbackInfo,
    ) -> Result<Authenticated, RealtimeError> {
        let url = config::realtime_websocket_url()?;
        info!(%url, "connecting WebSocket realtime fallback session");
        let (writer, reader) = websocket::split(url.as_str()).await?;
        let (sender, receiver) = mpsc::unbounded();
        let generation = self.next_generation();
        self.inner.streams.lock().await.clear();
        self.inner.pending.borrow_mut().clear();
        self.inner.session.lock().await.replace(ConnectedSession {
            generation,
            transport: ConnectedTransport::WebSocket(sender),
        });
        websocket::spawn_writer(
            url.to_string(),
            generation,
            writer,
            receiver,
            Some(self.clone()),
        );
        websocket::spawn_reader(
            url.to_string(),
            generation,
            reader,
            self.inner.inbound.clone(),
            self.inner.datagram_listeners.clone(),
            self.clone(),
        );

        let authenticated = self
            .request(
                RealtimeModule::Control,
                RealtimeKind::Control(ControlKind::Authenticate),
                Authenticate { access_token },
            )
            .await;
        let authenticated: Authenticated = match authenticated {
            Ok(authenticated) => authenticated,
            Err(error) => {
                self.clear_generation(generation).await;
                return Err(error);
            }
        };
        info!(%url, user_id = %authenticated.user.id, "WebSocket realtime fallback authenticated");
        self.set_connection_status(RealtimeConnectionStatus::Connected(
            RealtimeTransportKind::WebSocketFallback,
        ));
        self.publish_fallback_info(Some(fallback_info));

        Ok(authenticated)
    }
}

fn classify_transport_failure(error: &RealtimeError) -> WebTransportFallbackReason {
    let message = error.to_string().to_ascii_lowercase();
    if [
        "dns",
        "resolve",
        "lookup",
        "no such host",
        "name or service",
    ]
    .iter()
    .any(|fragment| message.contains(fragment))
    {
        WebTransportFallbackReason::Dns
    } else if ["tls", "certificate", "issuer", "x509", "trust manager"]
        .iter()
        .any(|fragment| message.contains(fragment))
    {
        WebTransportFallbackReason::Tls
    } else if ["webtransport", "quic", "transport", "connection"]
        .iter()
        .any(|fragment| message.contains(fragment))
    {
        WebTransportFallbackReason::Transport
    } else {
        WebTransportFallbackReason::Unknown
    }
}

fn elapsed_ms(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_transport_failures() {
        assert_eq!(
            classify_transport_failure(&RealtimeError::new("DNS lookup failed")),
            WebTransportFallbackReason::Dns
        );
        assert_eq!(
            classify_transport_failure(&RealtimeError::new("UnknownIssuer certificate error")),
            WebTransportFallbackReason::Tls
        );
        assert_eq!(
            classify_transport_failure(&RealtimeError::new("WebTransport connection rejected")),
            WebTransportFallbackReason::Transport
        );
        assert_eq!(
            classify_transport_failure(&RealtimeError::new("QUIC connection closed")),
            WebTransportFallbackReason::Transport
        );
    }
}

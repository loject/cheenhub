//! Подключение realtime transports.

use std::rc::Rc;

use cheenhub_contracts::realtime::{
    Authenticate, Authenticated, ControlKind, RealtimeKind, RealtimeModule,
};
use dioxus::prelude::{info, warn};
use futures_channel::mpsc;
use futures_util::FutureExt;
use futures_util::future::{Either, select};

use super::{ConnectedSession, ConnectedTransport, RealtimeHandle};
use crate::features::realtime::config;
use crate::features::realtime::error::RealtimeError;
use crate::features::realtime::status::{RealtimeConnectionStatus, RealtimeTransportKind};
use crate::features::realtime::{platform, websocket, webtransport};
use crate::features::runtime::sleep_ms;

const WEBTRANSPORT_CONNECT_TIMEOUT_MS: u32 = 1_500;

impl RealtimeHandle {
    /// Opens and authenticates the realtime session.
    pub(crate) async fn connect(
        &self,
        access_token: String,
    ) -> Result<Authenticated, RealtimeError> {
        let webtransport = self
            .connect_webtransport(access_token.clone())
            .boxed_local();
        let timeout = sleep_ms(WEBTRANSPORT_CONNECT_TIMEOUT_MS).boxed_local();
        let webtransport_result = match select(webtransport, timeout).await {
            Either::Left((result, _)) => result,
            Either::Right(((), _)) => Err(RealtimeError::new(format!(
                "WebTransport realtime connection timed out after {WEBTRANSPORT_CONNECT_TIMEOUT_MS} ms"
            ))),
        };

        match webtransport_result {
            Ok(authenticated) => Ok(authenticated),
            Err(webtransport_error) => {
                warn!(
                    %webtransport_error,
                    "WebTransport realtime connection failed; trying WebSocket fallback"
                );
                self.mark_connecting(RealtimeTransportKind::WebSocketFallback);
                self.connect_websocket(access_token)
                    .await
                    .map_err(|websocket_error| {
                        RealtimeError::new(format!(
                            "Failed to connect realtime session: WebTransport error: {webtransport_error}; WebSocket fallback error: {websocket_error}"
                        ))
                    })
            }
        }
    }

    async fn connect_webtransport(
        &self,
        access_token: String,
    ) -> Result<Authenticated, RealtimeError> {
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
        info!(%url, user_id = %authenticated.user.id, "WebTransport realtime authenticated");
        self.set_connection_status(RealtimeConnectionStatus::Connected(
            RealtimeTransportKind::WebTransport,
        ));
        webtransport::spawn_datagram_reader(
            session.clone(),
            generation,
            self.inner.datagram_listeners.clone(),
        );
        webtransport::spawn_connection_watcher(url.to_string(), session, generation, self.clone());

        Ok(authenticated)
    }

    async fn connect_websocket(
        &self,
        access_token: String,
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

        Ok(authenticated)
    }
}

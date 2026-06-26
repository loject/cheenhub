//! Подключение realtime transports.

use std::rc::Rc;

use cheenhub_contracts::realtime::{
    Authenticate, Authenticated, ControlKind, RealtimeKind, RealtimeModule,
};
use dioxus::prelude::{info, warn};
use futures_channel::mpsc;

use super::{ConnectedSession, ConnectedTransport, RealtimeHandle};
use crate::features::realtime::config;
use crate::features::realtime::error::RealtimeError;
use crate::features::realtime::status::{RealtimeConnectionStatus, RealtimeTransportKind};
use crate::features::realtime::{websocket, webtransport};

impl RealtimeHandle {
    /// Opens and authenticates the realtime session.
    pub(crate) async fn connect(
        &self,
        access_token: String,
    ) -> Result<Authenticated, RealtimeError> {
        match self.connect_webtransport(access_token.clone()).await {
            Ok(authenticated) => Ok(authenticated),
            Err(webtransport_error) => {
                warn!(
                    %webtransport_error,
                    "WebTransport realtime connection failed; trying WebSocket fallback"
                );
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
        let client = config::realtime_client()?;
        let url = config::realtime_url()?;
        info!(%url, "connecting WebTransport realtime session");
        let session = client.connect(url.clone()).await.map_err(|error| {
            RealtimeError::new(format!("Failed to connect realtime session: {error}"))
        })?;

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

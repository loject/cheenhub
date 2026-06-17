//! Каркас realtime WebTransport-клиента.

mod config;
mod error;
mod framing;
mod guards;
mod handle;
mod inbound;
mod provider;
mod status;
mod task;
mod websocket;
mod webtransport;

#[allow(unused_imports)]
pub(crate) use error::RealtimeError;
#[allow(unused_imports)]
pub(crate) use handle::RealtimeHandle;
pub(crate) use provider::RealtimeProvider;
pub(crate) use status::{RealtimeConnectionStatus, RealtimeTransportKind};

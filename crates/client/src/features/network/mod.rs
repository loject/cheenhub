//! Client network status feature.

mod connection_status_indicator;
pub(crate) mod realtime;

pub(crate) use connection_status_indicator::RealtimeConnectionStatusIndicator;

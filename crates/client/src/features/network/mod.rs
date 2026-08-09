//! Client network status feature.

mod connection_status_indicator;
mod quality;
pub(crate) mod realtime;
mod realtime_fallback_notice;

pub(crate) use connection_status_indicator::RealtimeConnectionStatusIndicator;
pub(crate) use quality::{NetworkQualityHandle, PingSample};
pub(crate) use realtime_fallback_notice::RealtimeFallbackNotice;

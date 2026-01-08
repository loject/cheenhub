/// SFU (Selective Forwarding Unit) implementation for WebRTC
/// 
/// This module provides the core SFU functionality for routing media streams
/// between publishers and consumers with minimal latency.

pub mod types;
pub mod router;
pub mod publisher;
pub mod consumer;

pub use router::SfuRouter;


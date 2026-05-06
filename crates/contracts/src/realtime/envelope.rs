//! Shared realtime envelope contracts.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use uuid::Uuid;

use super::control::ControlKind;
use super::network::NetworkKind;

/// Top-level realtime module namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeModule {
    /// Session lifecycle and diagnostic control messages.
    Control,
    /// Connection quality measurement messages.
    Network,
}

/// Typed realtime message kind wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RealtimeKind {
    /// Control module message kind.
    Control(ControlKind),
    /// Network module message kind.
    Network(NetworkKind),
}

impl RealtimeKind {
    /// Returns the module that owns this kind.
    pub fn module(self) -> RealtimeModule {
        match self {
            Self::Control(_) => RealtimeModule::Control,
            Self::Network(_) => RealtimeModule::Network,
        }
    }
}

impl Serialize for RealtimeKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Control(kind) => kind.serialize(serializer),
            Self::Network(kind) => kind.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for RealtimeKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        if let Ok(kind) = ControlKind::deserialize(value.clone()) {
            return Ok(Self::Control(kind));
        }
        if let Ok(kind) = NetworkKind::deserialize(value) {
            return Ok(Self::Network(kind));
        }

        Err(serde::de::Error::custom("unknown realtime kind"))
    }
}

/// Envelope used by every reliable realtime message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealtimeEnvelope {
    /// Module that owns the message.
    pub module: RealtimeModule,
    /// Module-local typed message kind.
    pub kind: RealtimeKind,
    /// Optional request identifier used for request-response correlation.
    pub request_id: Option<Uuid>,
    /// Module-owned JSON payload decoded by the receiving module.
    pub payload: Value,
}

impl RealtimeEnvelope {
    /// Creates a typed envelope from a serializable payload.
    pub fn new<T>(
        module: RealtimeModule,
        kind: RealtimeKind,
        request_id: Option<Uuid>,
        payload: T,
    ) -> Result<Self, serde_json::Error>
    where
        T: Serialize,
    {
        Ok(Self {
            module,
            kind,
            request_id,
            payload: serde_json::to_value(payload)?,
        })
    }

    /// Returns whether this envelope has a matching module/kind pair.
    pub fn has_matching_module_kind(&self) -> bool {
        self.kind.module() == self.module
    }
}

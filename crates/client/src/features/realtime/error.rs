//! Realtime client error types.

/// Realtime client error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RealtimeError {
    message: String,
}

impl RealtimeError {
    /// Creates a realtime client error.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for RealtimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RealtimeError {}

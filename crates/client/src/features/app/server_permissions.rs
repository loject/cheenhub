//! Per-server permissions context for the current user.

/// Context that describes what moderation actions the current user
/// may perform in the active server.
#[derive(Clone, Copy)]
pub(crate) struct ServerPermissionsContext {
    /// Whether the user can delete any message in a text room.
    pub(crate) can_moderate: bool,
    /// Whether the user can kick participants from voice rooms.
    pub(crate) can_kick_voice: bool,
}

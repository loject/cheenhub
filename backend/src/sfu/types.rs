use uuid::Uuid;

/// Unique identifier for a consumer connection
pub type ConsumerId = String;

/// Unique identifier for a track
pub type TrackId = String;

/// Generate a unique consumer ID
pub fn generate_consumer_id() -> ConsumerId {
    Uuid::new_v4().to_string()
}

/// Generate a unique track ID
pub fn generate_track_id() -> TrackId {
    Uuid::new_v4().to_string()
}

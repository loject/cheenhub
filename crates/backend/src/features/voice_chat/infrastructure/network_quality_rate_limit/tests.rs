//! Проверки ограничения частоты запросов качества сети.

use std::time::{Duration, Instant};

use uuid::Uuid;

use super::super::InMemoryVoicePresenceStore;

#[tokio::test]
async fn allows_normal_voice_cadence_and_rejects_faster_publications() {
    let store = InMemoryVoicePresenceStore::default();
    let stream_id = Uuid::new_v4();
    let started_at = Instant::now();

    assert!(
        store
            .allow_network_quality_publish_at(stream_id, started_at)
            .await
    );
    assert!(
        !store
            .allow_network_quality_publish_at(stream_id, started_at + Duration::from_millis(499),)
            .await
    );
    assert!(
        store
            .allow_network_quality_publish_at(stream_id, started_at + Duration::from_millis(750),)
            .await
    );
}

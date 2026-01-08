use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::sfu::publisher::Publisher;
use crate::sfu::consumer::Consumer;
use crate::sfu::types::{ConsumerId, TrackId, generate_consumer_id};

/// SfuRouter manages all publishers and consumers in the SFU
#[derive(Clone)]
pub struct SfuRouter {
    /// Map of user_id -> Publisher
    publishers: Arc<RwLock<HashMap<String, Arc<RwLock<Publisher>>>>>,
    /// Map of consumer_id -> Consumer
    consumers: Arc<RwLock<HashMap<ConsumerId, Arc<RwLock<Consumer>>>>>,
}

impl SfuRouter {
    /// Create a new SFU router
    pub fn new() -> Self {
        Self {
            publishers: Arc::new(RwLock::new(HashMap::new())),
            consumers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a publisher to the router
    pub async fn add_publisher(
        &self,
        user_id: String,
        username: String,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Creating publisher for user: {} ({})", username, user_id);
        
        // Create the publisher and get the SDP offer
        let (publisher, sdp_offer) = Publisher::create(user_id.clone(), username).await?;
        
        // Store the publisher
        let mut publishers = self.publishers.write().await;
        publishers.insert(user_id.clone(), publisher);
        
        tracing::info!("Publisher created for user: {}", user_id);
        Ok(sdp_offer)
    }

    /// Set the answer for a publisher
    pub async fn set_publisher_answer(
        &self,
        user_id: &str,
        sdp: String,
    ) -> Result<Option<TrackId>, Box<dyn std::error::Error + Send + Sync>> {
        let publishers = self.publishers.read().await;
        
        if let Some(publisher) = publishers.get(user_id) {
            let pub_read = publisher.read().await;
            pub_read.set_answer(sdp).await?;
            
            // Return the track ID if available (might not be available yet)
            Ok(pub_read.audio_track_id.clone())
        } else {
            Err(format!("Publisher not found for user: {}", user_id).into())
        }
    }

    /// Get track ID for a publisher (wait until available)
    pub async fn get_publisher_track_id(&self, user_id: &str, max_attempts: u32) -> Option<TrackId> {
        for _ in 0..max_attempts {
            let publishers = self.publishers.read().await;
            if let Some(publisher) = publishers.get(user_id) {
                let pub_read = publisher.read().await;
                if let Some(track_id) = pub_read.audio_track_id.clone() {
                    return Some(track_id);
                }
            }
            drop(publishers);
            
            // Wait before retrying
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        None
    }

    /// Remove a publisher from the router
    pub async fn remove_publisher(&self, user_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut publishers = self.publishers.write().await;
        
        if let Some(publisher) = publishers.remove(user_id) {
            let pub_read = publisher.read().await;
            pub_read.close().await?;
            tracing::info!("Publisher removed: {}", user_id);
        }
        
        Ok(())
    }

    /// Create a consumer for a subscriber to consume a publisher's track
    pub async fn add_consumer(
        &self,
        publisher_user_id: String,
        subscriber_user_id: String,
    ) -> Result<(ConsumerId, String), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            "Creating consumer for subscriber {} to consume publisher {}",
            subscriber_user_id,
            publisher_user_id
        );

        // Get the publisher
        let publishers = self.publishers.read().await;
        let publisher = publishers
            .get(&publisher_user_id)
            .ok_or(format!("Publisher not found: {}", publisher_user_id))?
            .clone();
        drop(publishers);

        // Get the publisher's audio track
        let pub_read = publisher.read().await;
        let audio_track = pub_read
            .audio_track
            .clone()
            .ok_or(format!("Publisher {} has no audio track yet", publisher_user_id))?;
        drop(pub_read);

        // Generate consumer ID
        let consumer_id = generate_consumer_id();

        // Create the consumer
        let (consumer, sdp_offer) = Consumer::create(
            consumer_id.clone(),
            publisher_user_id.clone(),
            subscriber_user_id.clone(),
            audio_track,
        )
        .await?;

        // Store the consumer
        let mut consumers = self.consumers.write().await;
        consumers.insert(consumer_id.clone(), consumer);

        tracing::info!(
            "Consumer {} created for subscriber {} <- publisher {}",
            consumer_id,
            subscriber_user_id,
            publisher_user_id
        );

        Ok((consumer_id, sdp_offer))
    }

    /// Set the answer for a consumer
    pub async fn set_consumer_answer(
        &self,
        consumer_id: &str,
        sdp: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let consumers = self.consumers.read().await;
        
        if let Some(consumer) = consumers.get(consumer_id) {
            let cons_read = consumer.read().await;
            cons_read.set_answer(sdp).await?;
            Ok(())
        } else {
            Err(format!("Consumer not found: {}", consumer_id).into())
        }
    }

    /// Remove all consumers for a specific subscriber
    pub async fn remove_consumers_for_subscriber(&self, subscriber_user_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut consumers = self.consumers.write().await;
        let mut to_remove = Vec::new();

        for (consumer_id, consumer) in consumers.iter() {
            let cons_read = consumer.read().await;
            if cons_read.subscriber_user_id == subscriber_user_id {
                to_remove.push(consumer_id.clone());
            }
        }

        for consumer_id in to_remove {
            if let Some(consumer) = consumers.remove(&consumer_id) {
                let cons_read = consumer.read().await;
                cons_read.close().await?;
                tracing::info!("Consumer removed: {}", consumer_id);
            }
        }

        Ok(())
    }

    /// Add ICE candidate to publisher
    pub async fn add_publisher_ice_candidate(
        &self,
        user_id: &str,
        candidate: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let publishers = self.publishers.read().await;
        
        if let Some(publisher) = publishers.get(user_id) {
            let pub_read = publisher.read().await;
            pub_read.add_ice_candidate(candidate).await?;
            Ok(())
        } else {
            Err(format!("Publisher not found: {}", user_id).into())
        }
    }

    /// Add ICE candidate to consumer
    pub async fn add_consumer_ice_candidate(
        &self,
        consumer_id: &str,
        candidate: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let consumers = self.consumers.read().await;
        
        if let Some(consumer) = consumers.get(consumer_id) {
            let cons_read = consumer.read().await;
            cons_read.add_ice_candidate(candidate).await?;
            Ok(())
        } else {
            Err(format!("Consumer not found: {}", consumer_id).into())
        }
    }
}

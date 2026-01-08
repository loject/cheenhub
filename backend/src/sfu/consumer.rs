use std::sync::Arc;
use tokio::sync::RwLock;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::{TrackLocal, TrackLocalWriter};
use webrtc::track::track_remote::TrackRemote;
use webrtc::util::{Marshal, MarshalSize};

use crate::sfu::types::ConsumerId;

/// Consumer represents a peer that consumes (receives) media tracks from the SFU
pub struct Consumer {
    pub consumer_id: ConsumerId,
    pub _publisher_user_id: String,
    pub subscriber_user_id: String,
    pub peer_connection: Arc<RTCPeerConnection>,
    pub _audio_track: Arc<TrackLocalStaticRTP>,
}

impl Consumer {
    /// Create a new Consumer with a WebRTC PeerConnection and track
    pub async fn create(
        consumer_id: ConsumerId,
        publisher_user_id: String,
        subscriber_user_id: String,
        publisher_track: Arc<TrackRemote>,
    ) -> Result<(Arc<RwLock<Self>>, String), Box<dyn std::error::Error + Send + Sync>> {
        // Create a MediaEngine for audio only
        let mut media_engine = MediaEngine::default();
        
        // Register default codecs (includes Opus for audio)
        media_engine.register_default_codecs()?;

        // Create the API with the MediaEngine
        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .build();

        // Configure ICE servers (STUN)
        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            }],
            ..Default::default()
        };

        // Create PeerConnection
        let peer_connection = Arc::new(api.new_peer_connection(config).await?);

        // Create a local track to send to the consumer
        let audio_track = Arc::new(TrackLocalStaticRTP::new(
            publisher_track.codec().capability,
            format!("audio-{}", consumer_id),
            format!("stream-{}", publisher_user_id),
        ));

        // Add track to peer connection
        let _rtp_sender = peer_connection
            .add_track(Arc::clone(&audio_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await?;

        let consumer = Arc::new(RwLock::new(Consumer {
            consumer_id: consumer_id.clone(),
            _publisher_user_id: publisher_user_id.clone(),
            subscriber_user_id: subscriber_user_id.clone(),
            peer_connection: Arc::clone(&peer_connection),
            _audio_track: Arc::clone(&audio_track),
        }));

        // Handle peer connection state changes
        let consumer_id_clone = consumer_id.clone();
        peer_connection.on_peer_connection_state_change(Box::new(move |state: RTCPeerConnectionState| {
            let consumer_id = consumer_id_clone.clone();
            Box::pin(async move {
                tracing::info!("Consumer {} peer connection state: {}", consumer_id, state);
            })
        }));

        // Start forwarding RTP packets from publisher track to consumer track
        let audio_track_clone = Arc::clone(&audio_track);
        let consumer_id_clone = consumer_id.clone();
        tokio::spawn(async move {
            if let Err(e) = forward_rtp_packets(publisher_track, audio_track_clone, consumer_id_clone).await {
                tracing::error!("Error forwarding RTP packets: {}", e);
            }
        });

        // Create and set local description (offer)
        let offer = peer_connection.create_offer(None).await?;
        peer_connection.set_local_description(offer).await?;

        // Wait for ICE gathering to complete
        let mut gather_complete = peer_connection.gathering_complete_promise().await;
        let _ = gather_complete.recv().await;

        // Get the complete SDP offer
        let local_desc = peer_connection
            .local_description()
            .await
            .ok_or("Failed to get local description")?;

        let sdp_offer = local_desc.sdp;

        tracing::info!("Consumer {} created for publisher {}", consumer_id, publisher_user_id);

        Ok((consumer, sdp_offer))
    }

    /// Set the remote SDP answer from the client
    pub async fn set_answer(&self, sdp: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let answer = RTCSessionDescription::answer(sdp)?;
        self.peer_connection.set_remote_description(answer).await?;
        
        tracing::info!("Consumer {} answer set successfully", self.consumer_id);
        Ok(())
    }

    /// Add an ICE candidate
    pub async fn add_ice_candidate(&self, candidate: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
        
        let ice_candidate = RTCIceCandidateInit {
            candidate: candidate.clone(),
            ..Default::default()
        };
        
        self.peer_connection.add_ice_candidate(ice_candidate).await?;
        tracing::debug!("Consumer {} added ICE candidate", self.consumer_id);
        Ok(())
    }

    /// Close the consumer connection
    pub async fn close(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.peer_connection.close().await?;
        tracing::info!("Consumer {} closed", self.consumer_id);
        Ok(())
    }
}

/// Forward RTP packets from publisher track to consumer track
async fn forward_rtp_packets(
    publisher_track: Arc<TrackRemote>,
    consumer_track: Arc<TrackLocalStaticRTP>,
    consumer_id: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Starting RTP forwarding for consumer {}", consumer_id);
    
    let mut packet_count = 0u64;
    
    loop {
        // Read RTP packet from publisher track
        let (rtp_packet, _) = match publisher_track.read_rtp().await {
            Ok(packet) => packet,
            Err(e) => {
                tracing::warn!("Consumer {} RTP read error: {}", consumer_id, e);
                break;
            }
        };

        packet_count += 1;
        
        // Serialize RTP packet to bytes
        let mut buf = vec![0u8; rtp_packet.marshal_size()];
        if let Err(e) = rtp_packet.marshal_to(&mut buf) {
            tracing::warn!("Consumer {} RTP marshal error: {}", consumer_id, e);
            break;
        }
        
        // Forward packet bytes to consumer track
        if let Err(e) = consumer_track.write(&buf).await {
            tracing::warn!("Consumer {} RTP write error: {}", consumer_id, e);
            break;
        }

        // Log progress every 1000 packets
        if packet_count % 1000 == 0 {
            tracing::debug!("Consumer {} forwarded {} packets", consumer_id, packet_count);
        }
    }

    tracing::info!("RTP forwarding stopped for consumer {} after {} packets", consumer_id, packet_count);
    Ok(())
}

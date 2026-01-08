use std::sync::Arc;
use tokio::sync::RwLock;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_transceiver_direction::RTCRtpTransceiverDirection;
use webrtc::rtp_transceiver::RTCRtpTransceiverInit;
use webrtc::track::track_remote::TrackRemote;

use crate::sfu::types::{TrackId, generate_track_id};

/// Publisher represents a peer that publishes media tracks to the SFU
pub struct Publisher {
    pub user_id: String,
    pub _username: String,
    pub peer_connection: Arc<RTCPeerConnection>,
    pub audio_track_id: Option<TrackId>,
    pub audio_track: Option<Arc<TrackRemote>>,
}

impl Publisher {
    /// Create a new Publisher with a WebRTC PeerConnection
    pub async fn create(
        user_id: String,
        username: String,
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

        let publisher = Arc::new(RwLock::new(Publisher {
            user_id: user_id.clone(),
            _username: username,
            peer_connection: Arc::clone(&peer_connection),
            audio_track_id: None,
            audio_track: None,
        }));

        // Handle incoming tracks
        let publisher_clone = Arc::clone(&publisher);
        peer_connection.on_track(Box::new(move |track, _receiver, _transceiver| {
            let publisher = Arc::clone(&publisher_clone);
            Box::pin(async move {
                tracing::info!("Publisher received track: kind={:?}", track.kind());
                
                let track_id = generate_track_id();
                let mut pub_write = publisher.write().await;
                pub_write.audio_track_id = Some(track_id.clone());
                pub_write.audio_track = Some(track);
                
                tracing::info!("Publisher track registered with ID: {}", track_id);
            })
        }));

        // Handle peer connection state changes
        let user_id_clone = user_id.clone();
        peer_connection.on_peer_connection_state_change(Box::new(move |state: RTCPeerConnectionState| {
            let user_id = user_id_clone.clone();
            Box::pin(async move {
                tracing::info!("Publisher {} peer connection state: {}", user_id, state);
            })
        }));

        // Add audio transceiver to enable audio media section in SDP
        // This is required for browser to generate proper ICE credentials in answer
        tracing::info!("Adding recvonly audio transceiver to publisher connection");
        
        let transceiver_init = RTCRtpTransceiverInit {
            direction: RTCRtpTransceiverDirection::Recvonly,
            send_encodings: vec![],
        };
        
        peer_connection
            .add_transceiver_from_kind(
                webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Audio,
                Some(transceiver_init),
            )
            .await?;

        tracing::info!("Audio transceiver added, creating offer");

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

        Ok((publisher, sdp_offer))
    }

    /// Set the remote SDP answer from the client
    pub async fn set_answer(&self, sdp: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let answer = RTCSessionDescription::answer(sdp)?;
        self.peer_connection.set_remote_description(answer).await?;
        
        tracing::info!("Publisher {} answer set successfully", self.user_id);
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
        tracing::debug!("Publisher {} added ICE candidate", self.user_id);
        Ok(())
    }

    /// Close the publisher connection
    pub async fn close(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.peer_connection.close().await?;
        tracing::info!("Publisher {} closed", self.user_id);
        Ok(())
    }
}

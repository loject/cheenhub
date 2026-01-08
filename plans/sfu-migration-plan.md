# SFU Migration Plan: P2P Mesh → Pure Rust SFU

**Version:** 1.0  
**Date:** 2026-01-08  
**Status:** Planning Phase

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Overview](#architecture-overview)
3. [Technology Stack](#technology-stack)
4. [Migration Phases](#migration-phases)
5. [Backend Architecture](#backend-architecture)
6. [Frontend Changes](#frontend-changes)
7. [Risk Assessment & Mitigation](#risk-assessment--mitigation)
8. [Success Criteria](#success-criteria)

---

## Executive Summary

### Current State

**Topology:** P2P Mesh
- Each participant establishes direct WebRTC connection with every other participant
- N participants = N*(N-1)/2 connections total
- Server only relays signaling messages (SDP offers/answers, ICE candidates)
- No server-side media processing

**Scalability Limitations:**
- With 5 participants: 10 connections (manageable)
- With 10 participants: 45 connections (problematic)
- With 20 participants: 190 connections (impractical)
- Exponential growth in bandwidth and CPU usage per client

**Current Tech Stack:**
- Backend: Axum 0.7 + Tokio async runtime
- Frontend: Dioxus + web-sys WebRTC API
- Protocol: WebSocket with JSON messages
- Media: Audio-only (Opus codec, 48kHz, 10ms latency target)

### Target State

**Topology:** SFU (Selective Forwarding Unit)
- Each participant establishes 1 WebRTC connection to server
- N participants = N connections total (linear scaling)
- Server forwards media streams selectively
- Server acts as media router, not transcoder

**Benefits:**
- Linear bandwidth scaling on clients: O(1) vs O(N)
- Centralized quality control and monitoring
- Foundation for advanced features (recording, mixing, layout control)
- Supports 50+ participants with proper optimization

**Pure Rust Approach:**
- webrtc-rs for WebRTC stack implementation
- No external media servers (Mediasoup, Janus, etc.)
- Full control over media pipeline
- Low-latency optimizations at every layer

### Migration Strategy

**4-Phase Gradual Migration:**
1. **Phase 1:** Minimal viable SFU (audio forwarding only)
2. **Phase 2:** Bandwidth adaptation & quality control
3. **Phase 3:** Simulcast support for bandwidth efficiency
4. **Phase 4:** Production optimizations & monitoring

Each phase delivers measurable value and can be deployed independently.

---

## Architecture Overview

### Current Architecture (P2P Mesh)

```
┌─────────────┐
│   Client A  │
│  (Browser)  │────┐
└─────────────┘    │
       │           │ WebRTC P2P
       │ WS        │ (Direct)
       │           │
┌──────▼──────┐    │
│   Backend   │    │
│  (Signaling │    │
│    Relay)   │    │
└──────┬──────┘    │
       │ WS        │
       │           │
┌──────▼──────┐    │
│   Client B  │────┘
│  (Browser)  │
└─────────────┘

- Backend: Only relays signaling (Offer/Answer/ICE)
- Media: Direct browser-to-browser (P2P)
- Connections: N*(N-1)/2 for N clients
```

### Target Architecture (SFU)

```
                    ┌─────────────────────────┐
                    │    Backend (SFU)        │
                    │  ┌──────────────────┐   │
┌──────────────┐    │  │  WebSocket       │   │
│  Client A    │    │  │  Signaling       │   │
│  (Browser)   │◄───┼──┤  Handler         │   │
│              │    │  └────────┬─────────┘   │
│  - Publisher │    │           │             │
│    (Send)    │────┼───────────┼────────────►│
│  - Consumer  │    │           │             │
│    (Receive) │◄───┼───────────┤             │
└──────────────┘    │           │             │
                    │  ┌────────▼─────────┐   │
                    │  │  SFU Router      │   │
┌──────────────┐    │  │                  │   │
│  Client B    │    │  │ - PeerConnection │   │
│  (Browser)   │◄───┼──┤   Management     │   │
│              │    │  │ - Track          │   │
│  - Publisher │────┼──┤   Forwarding     │   │
│  - Consumer  │◄───┼──┤ - Bandwidth      │   │
└──────────────┘    │  │   Adaptation     │   │
                    │  └──────────────────┘   │
┌──────────────┐    │                         │
│  Client C    │◄───┼─────────────────────────┤
└──────────────┘    └─────────────────────────┘

- Backend: Manages WebRTC connections + forwards media
- Media: Client → Server → Clients (star topology)
- Connections: N for N clients
```

### Data Flow: Audio Publishing & Consuming

```
Publisher (Client A) Flow:
=========================
1. getUserMedia() → Local MediaStream
2. Create RTCPeerConnection to SFU
3. addTrack(audioTrack) → PeerConnection
4. Send Offer SDP via WebSocket
5. Receive Answer SDP from SFU
6. ICE negotiation completes
7. Audio RTP packets → SFU

SFU Processing:
===============
1. Receive RTP packets from Client A
2. Track management: Store track reference
3. For each consumer (Client B, C, ...):
   - Forward RTP packets without transcoding
   - Apply bandwidth adaptation if needed
   - Handle packet loss, jitter buffers

Consumer (Client B) Flow:
========================
1. Create RTCPeerConnection to SFU
2. Subscribe to Client A's audio track
3. Receive track via ontrack event
4. Receive Audio RTP packets from SFU
5. Decode & play audio stream
```

---

## Technology Stack

### Core Dependencies

#### Backend (New Dependencies)

```toml
[dependencies]
# Existing
axum = { version = "0.7", features = ["ws"] }
tokio = { version = "1.42", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.11", features = ["v4", "serde"] }
tracing = "0.1"
tracing-subscriber = "0.3"

# New for SFU
webrtc = "0.11"  # Core WebRTC implementation
tokio-util = { version = "0.7", features = ["codec"] }
bytes = "1.5"
async-trait = "0.1"

# Optional (for advanced features)
parking_lot = "0.12"  # Faster RwLock
dashmap = "6.0"       # Concurrent HashMap
```

**webrtc crate breakdown:**
- `webrtc::api::API` - Main WebRTC API entry point
- `webrtc::api::media_engine::MediaEngine` - Codec configuration
- `webrtc::api::interceptor_registry::InterceptorRegistry` - RTCP/RTP interceptors
- `webrtc::peer_connection::RTCPeerConnection` - Peer connection management
- `webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver` - Receive RTP streams
- `webrtc::rtp_transceiver::rtp_sender::RTCRtpSender` - Send RTP streams
- `webrtc::track::track_local::TrackLocal` - Local track abstraction
- `webrtc::track::track_remote::TrackRemote` - Remote track abstraction

#### Frontend (No Major Changes)

```toml
[dependencies]
# Existing (all remain the same)
dioxus = { version = "0.7.2", features = ["web"] }
web-sys = { version = "0.3", features = [
    "RtcPeerConnection", "MediaStream", # ... etc
] }
```

**Frontend changes are minimal:**
- Update signaling message types
- Change connection flow (publisher/consumer model)
- UI remains largely unchanged

### Justification

**Why webrtc-rs?**

1. **Pure Rust:** Type safety, memory safety, no FFI overhead
2. **Async-first:** Built on tokio, integrates seamlessly with Axum
3. **Production-ready:** Used in real-world SFU implementations
4. **Full control:** Access to RTP/RTCP layers for optimization
5. **Active development:** Regular updates, good community support

**Alternatives Considered:**

| Solution | Pros | Cons | Decision |
|----------|------|------|----------|
| Mediasoup | Battle-tested, feature-rich | Node.js, complex FFI | ❌ Not Pure Rust |
| Janus | Stable, well-documented | C/C++, plugin system | ❌ Not Pure Rust |
| Jitsi Videobridge | Proven at scale | Java, heavyweight | ❌ Not Pure Rust |
| webrtc-rs | Pure Rust, full control | Younger ecosystem | ✅ Selected |

---

## Migration Phases

### Phase 1: Minimal Viable SFU (MVP)

**Goal:** Replace P2P mesh with basic SFU forwarding

**Scope:**
- Single WebRTC connection per client to server
- Audio track publishing (client → server)
- Audio track consuming (server → client)
- Basic track management and routing
- No bandwidth adaptation yet

**Features:**
- ✅ Client publishes audio to SFU
- ✅ SFU forwards audio to all other clients
- ✅ Single connection per participant
- ✅ Maintains existing room management
- ✅ Opus codec support (48kHz)
- ❌ No bandwidth adaptation
- ❌ No simulcast
- ❌ No recording

**Target Latency:** <100ms end-to-end (similar to current)

**Deliverables:**
1. SFU router core module
2. WebRTC peer connection management
3. Track forwarding logic
4. Updated signaling protocol
5. Frontend publisher/consumer implementation

**Success Criteria:**
- 5 participants in a room with stable audio
- Latency ≤ 100ms (glass-to-glass)
- No audio dropouts or glitches
- CPU usage on server: <20% for 5 participants

**Testing Strategy:**
- Unit tests for router logic
- Integration tests with synthetic RTP streams
- Manual testing with 2-5 participants
- Latency measurement tools

**Estimated Duration:** 2-3 weeks

---

### Phase 2: Bandwidth Adaptation

**Goal:** Add intelligent bandwidth management for varying network conditions

**Scope:**
- Monitor bandwidth usage per client
- RTCP feedback processing (sender/receiver reports)
- Dynamic bitrate adjustment
- Packet loss detection and mitigation
- Jitter buffer optimization

**Features:**
- ✅ RTCP statistics collection
- ✅ Bandwidth estimation (REMB, Transport-CC)
- ✅ Dynamic bitrate signaling to clients
- ✅ Selective forwarding based on available bandwidth
- ✅ Packet loss monitoring and alerts
- ❌ No simulcast yet

**Enhancements:**
- Adapt to poor network conditions gracefully
- Prioritize audio quality over bandwidth
- Provide real-time quality metrics to clients

**Deliverables:**
1. RTCP interceptor implementation
2. Bandwidth estimator module
3. Quality adaptation logic
4. Enhanced statistics API
5. Client-side bandwidth reporting

**Success Criteria:**
- Graceful degradation on 2G/3G networks
- No audio dropouts with 20% packet loss
- Automatic recovery from network hiccups
- Quality metrics visible in UI

**Estimated Duration:** 2 weeks

---

### Phase 3: Simulcast Support

**Goal:** Enable multiple quality layers for bandwidth efficiency

**Scope:**
- Client sends multiple encoding layers (high/medium/low)
- SFU selects appropriate layer per consumer
- Layer switching based on network conditions
- Spatial/temporal scalability

**Features:**
- ✅ Multi-layer encoding on publisher
- ✅ Layer selection on SFU
- ✅ Dynamic layer switching
- ✅ Bandwidth savings for mobile clients
- ✅ Improved scalability (50+ participants)

**Technical Details:**
- Opus codec: Simulcast via multiple streams
- Use SDP negotiation for simulcast setup
- Layer switching without glitches (smooth transitions)

**Deliverables:**
1. Simulcast negotiation logic
2. Layer switching algorithm
3. Client-side simulcast encoding
4. SFU layer selector
5. Performance monitoring

**Success Criteria:**
- 3 quality layers (64kbps, 32kbps, 16kbps)
- <100ms layer switch time
- 50% bandwidth savings on mobile
- Support 20+ participants smoothly

**Estimated Duration:** 2-3 weeks

---

### Phase 4: Production Optimizations

**Goal:** Harden system for production deployment

**Scope:**
- Performance optimization (CPU, memory, network)
- Monitoring and observability
- Error recovery and fault tolerance
- Security hardening
- Documentation and deployment guides

**Features:**
- ✅ Prometheus metrics export
- ✅ Structured logging (tracing)
- ✅ Health check endpoints
- ✅ Graceful shutdown and reconnection
- ✅ Rate limiting and DoS protection
- ✅ TLS/WSS support
- ✅ Horizontal scaling considerations
- ✅ Load testing results

**Deliverables:**
1. Metrics collection system
2. Distributed tracing setup
3. Production configuration templates
4. Deployment automation (Docker, K8s)
5. Monitoring dashboards
6. Security audit and fixes
7. Performance benchmarks
8. Operations runbook

**Success Criteria:**
- 99.9% uptime over 1 week
- <50ms p99 latency under load
- Support 50+ concurrent participants
- CPU usage <60% at full capacity
- Automatic recovery from crashes
- Full observability stack

**Estimated Duration:** 2-3 weeks

---

## Backend Architecture

### Module Structure

```
backend/src/
├── main.rs                 # Entry point, server setup
├── signaling/             # WebSocket signaling
│   ├── mod.rs
│   ├── handler.rs         # WebSocket connection handler
│   ├── messages.rs        # Protocol message definitions
│   └── room_manager.rs    # Room lifecycle management
├── sfu/                   # SFU core logic
│   ├── mod.rs
│   ├── router.rs          # Main SFU router
│   ├── peer_connection.rs # WebRTC peer management
│   ├── track_manager.rs   # Track lifecycle & forwarding
│   ├── publisher.rs       # Publisher abstraction
│   ├── consumer.rs        # Consumer abstraction
│   └── transport.rs       # Transport management
├── media/                 # Media processing
│   ├── mod.rs
│   ├── codec.rs           # Codec configuration (Opus)
│   ├── rtp_forwarder.rs   # RTP packet forwarding
│   └── bandwidth.rs       # Bandwidth estimation (Phase 2)
├── stats/                 # Statistics & monitoring
│   ├── mod.rs
│   ├── collector.rs       # Metrics collection
│   └── rtcp_processor.rs  # RTCP feedback processing
└── config.rs              # Configuration management
```

### Key Components

#### 1. SFU Router (`sfu/router.rs`)

**Responsibilities:**
- Create and manage WebRTC API instance
- Handle room-level routing decisions
- Coordinate publishers and consumers
- Track management (add/remove/forward)

**Key Types:**

```rust
pub struct SfuRouter {
    api: Arc<webrtc::api::API>,
    rooms: Arc<RwLock<HashMap<String, Room>>>,
    publishers: Arc<RwLock<HashMap<String, Publisher>>>,
    consumers: Arc<RwLock<HashMap<String, Vec<Consumer>>>>,
}

pub struct Room {
    id: String,
    participants: HashSet<String>,
    created_at: Instant,
}

impl SfuRouter {
    pub async fn new() -> Result<Self>;
    pub async fn create_publisher(&self, user_id: String, room_id: String) -> Result<Publisher>;
    pub async fn create_consumer(&self, user_id: String, publisher_id: String) -> Result<Consumer>;
    pub async fn forward_track(&self, publisher_id: String, track: Arc<TrackRemote>) -> Result<()>;
    pub async fn remove_participant(&self, user_id: String) -> Result<()>;
}
```

#### 2. Publisher (`sfu/publisher.rs`)

**Responsibilities:**
- Manage client's publishing PeerConnection
- Receive media tracks from client
- Handle SDP offer/answer for publishing
- Track lifecycle management

**Key Types:**

```rust
pub struct Publisher {
    user_id: String,
    room_id: String,
    peer_connection: Arc<RTCPeerConnection>,
    tracks: Arc<RwLock<Vec<Arc<TrackRemote>>>>,
    state: Arc<RwLock<PublisherState>>,
}

pub enum PublisherState {
    Connecting,
    Connected,
    Disconnected,
    Failed,
}

impl Publisher {
    pub async fn new(user_id: String, room_id: String, api: Arc<API>) -> Result<Self>;
    pub async fn handle_offer(&self, sdp: String) -> Result<String>; // Returns answer SDP
    pub async fn add_ice_candidate(&self, candidate: String) -> Result<()>;
    pub async fn get_tracks(&self) -> Vec<Arc<TrackRemote>>;
    pub async fn close(&self) -> Result<()>;
}
```

#### 3. Consumer (`sfu/consumer.rs`)

**Responsibilities:**
- Manage client's consuming PeerConnection
- Send media tracks to client
- Handle SDP offer/answer for consuming
- Track subscription management

**Key Types:**

```rust
pub struct Consumer {
    user_id: String,
    subscribed_to: String, // Publisher user_id
    peer_connection: Arc<RTCPeerConnection>,
    senders: Arc<RwLock<Vec<Arc<RTCRtpSender>>>>,
    state: Arc<RwLock<ConsumerState>>,
}

pub enum ConsumerState {
    Connecting,
    Connected,
    Paused,
    Disconnected,
}

impl Consumer {
    pub async fn new(user_id: String, subscribed_to: String, api: Arc<API>) -> Result<Self>;
    pub async fn add_track(&self, track: Arc<TrackLocal>) -> Result<()>;
    pub async fn create_offer(&self) -> Result<String>; // Returns offer SDP
    pub async fn handle_answer(&self, sdp: String) -> Result<()>;
    pub async fn add_ice_candidate(&self, candidate: String) -> Result<()>;
    pub async fn pause(&self) -> Result<()>;
    pub async fn resume(&self) -> Result<()>;
    pub async fn close(&self) -> Result<()>;
}
```

#### 4. Track Manager (`sfu/track_manager.rs`)

**Responsibilities:**
- Forward RTP packets from publisher to consumers
- Handle track lifecycle events
- Implement selective forwarding logic
- Buffer management

**Key Types:**

```rust
pub struct TrackManager {
    forwarders: Arc<RwLock<HashMap<String, Vec<Arc<TrackForwarder>>>>>,
}

pub struct TrackForwarder {
    publisher_track: Arc<TrackRemote>,
    consumer_track: Arc<dyn TrackLocal + Send + Sync>,
    stats: Arc<RwLock<ForwardingStats>>,
}

impl TrackManager {
    pub async fn forward_track(
        &self,
        publisher_id: String,
        publisher_track: Arc<TrackRemote>,
        consumers: Vec<Arc<dyn TrackLocal + Send + Sync>>,
    ) -> Result<()>;
    
    pub async fn stop_forwarding(&self, publisher_id: String) -> Result<()>;
}
```

### WebRTC API Configuration

```rust
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;

pub async fn create_webrtc_api() -> Result<API> {
    let mut media_engine = MediaEngine::default();
    
    // Register Opus codec for audio
    media_engine.register_default_codecs()?;
    
    // Optimize for low latency
    // - Set Opus parameters: 48kHz, stereo, FEC enabled
    // - Configure jitter buffer: minimal buffering
    
    let mut interceptor_registry = Registry::new();
    register_default_interceptors(&mut media_engine, &mut interceptor_registry)?;
    
    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(interceptor_registry)
        .build();
    
    Ok(api)
}

pub fn create_peer_connection_config() -> RTCConfiguration {
    RTCConfiguration {
        ice_servers: vec![
            RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_string()],
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}
```

### Threading Model & Async Runtime

**Tokio Runtime Configuration:**
- Multi-threaded runtime (worker threads = CPU cores)
- Dedicated thread pool for media processing (optional for Phase 4)
- Async task per peer connection
- Channel-based communication between components

**Concurrency Strategy:**
```rust
// Main router with async RwLock for infrequent writes
pub struct SfuRouter {
    rooms: Arc<RwLock<HashMap<String, Room>>>,
}

// High-frequency forwarding with lock-free structures (Phase 4)
pub struct TrackManager {
    forwarders: Arc<DashMap<String, Vec<Arc<TrackForwarder>>>>,
}
```

### Memory Management

**Considerations:**
- RTP packets: Pool allocation for reduced GC pressure
- Track buffers: Bounded queues to prevent memory leaks
- Connection cleanup: Ensure all resources freed on disconnect
- Weak references: Use `Weak<T>` for back-references to avoid cycles

**Example:**
```rust
// Bounded channel for RTP packets
const RTP_BUFFER_SIZE: usize = 1000;
let (tx, rx) = tokio::sync::mpsc::channel(RTP_BUFFER_SIZE);

// Cleanup on disconnect
impl Drop for Publisher {
    fn drop(&mut self) {
        // Close peer connection
        // Stop all track readers
        // Remove from router registry
    }
}
```

---

## Frontend Changes

### Connection Model: Publisher/Consumer Pattern

**Current (P2P Mesh):**
```rust
// One PeerConnection per remote participant
let peer_connections = use_signal(|| HashMap::<String, RtcPeerConnection>::new());

// When user joins: create connection, send offer
for participant in new_participants {
    let pc = create_peer_connection(participant.id)?;
    pc.create_offer()?;
    peer_connections.write().insert(participant.id, pc);
}
```

**New (SFU):**
```rust
// Single Publisher connection (send audio)
let publisher_connection = use_signal(|| None::<RtcPeerConnection>);

// Single Consumer connection (receive all audio)
let consumer_connection = use_signal(|| None::<RtcPeerConnection>);

// Publishing flow
async fn setup_publisher(local_stream: MediaStream) -> Result<()> {
    let pc = create_rtc_peer_connection()?;
    
    // Add local audio track
    for track in local_stream.get_tracks() {
        pc.add_track(&track, &local_stream)?;
    }
    
    // Create offer, send to server
    let offer = pc.create_offer().await?;
    pc.set_local_description(&offer).await?;
    
    send_to_server(ClientMessage::PublishOffer {
        sdp: offer.sdp,
    })?;
    
    publisher_connection.set(Some(pc));
    Ok(())
}

// Consuming flow
async fn setup_consumer() -> Result<()> {
    let pc = create_rtc_peer_connection()?;
    
    // Set up ontrack handler to receive all remote tracks
    pc.set_ontrack(move |event: RtcTrackEvent| {
        let streams = event.streams();
        let track = event.track();
        
        // Play audio for each received track
        play_remote_audio(track, track_id);
    });
    
    // Request subscription from server
    send_to_server(ClientMessage::Subscribe {
        room_id: room_id.clone(),
    })?;
    
    consumer_connection.set(Some(pc));
    Ok(())
}
```

### Signaling Flow Changes

**Current Flow:**
1. Client A joins room
2. Client B joins room
3. Client B creates offer → Server → Client A
4. Client A creates answer → Server → Client B
5. ICE candidates exchanged bidirectionally

**New SFU Flow:**

```
Publishing Flow:
================
Client                          Server (SFU)
  │                                │
  ├─── Register/Join Room ────────►│
  │◄─── RoomJoined ────────────────┤
  │                                │
  ├─── PublishOffer (SDP) ────────►│
  │                                ├─ Create Publisher
  │                                ├─ Set RemoteDescription
  │◄─── PublishAnswer (SDP) ───────┤
  │                                │
  ├─── IceCandidate ──────────────►│
  │◄─── IceCandidate ──────────────┤
  │                                │
  ├─── RTP Audio Packets ─────────►│
  │                                │

Consuming Flow:
===============
Client                          Server (SFU)
  │                                │
  ├─── Subscribe (room_id) ───────►│
  │                                ├─ Create Consumer
  │                                ├─ Add all tracks
  │◄─── ConsumeOffer (SDP) ────────┤
  │                                │
  ├─── ConsumeAnswer (SDP) ───────►│
  │                                │
  ├─── IceCandidate ──────────────►│
  │◄─── IceCandidate ──────────────┤
  │                                │
  │◄─── RTP Audio Packets ──────────┤
  │                                │

Track Events:
=============
  │◄─── TrackAdded {               │
  │      track_id,                 │
  │      user_id,                  │  
  │      track_type: "audio"       │
  │     } ──────────────────────────┤
  │                                │
  │◄─── TrackRemoved {             │
  │      track_id                  │
  │     } ──────────────────────────┤
```

### Updated WebRTC Setup

**Key Changes:**
1. Single publisher PeerConnection (upload direction)
2. Single consumer PeerConnection (download direction)
3. Server initiates consumer offer (role reversal)
4. Track mapping: server-side track_id → user_id

**Implementation Example:**

```rust
// Setup publisher when microphone is granted
let request_microphone = move |_| {
    spawn_local(async move {
        let stream = get_user_media().await?;
        
        // Setup publisher immediately
        setup_publisher(stream.clone()).await?;
        
        media_stream.set(Some(stream));
        mic_status.set(MicStatus::Allowed);
    });
};

// Setup consumer when joining room
let join_room = move |_| {
    spawn_local(async move {
        // Join room via signaling
        send_message(ClientMessage::JoinRoom { room_id })?;
        
        // Setup consumer to receive tracks
        setup_consumer().await?;
    });
};

// Handle incoming tracks
let ontrack = Closure::wrap(Box::new(move |event: RtcTrackEvent| {
    let track = event.track();
    let streams = event.streams();
    
    // Extract track metadata from stream ID or SDP
    // Server embeds user_id in stream ID: "audio-{user_id}"
    if let Some(stream) = streams.get(0).dyn_into::<MediaStream>().ok() {
        let stream_id = stream.id();
        if let Some(user_id) = extract_user_id_from_stream(&stream_id) {
            // Play audio for this user
            play_remote_audio(track, user_id);
        }
    }
}));
```

### UI Changes

**Minimal UI impact:**
- Participant list: No changes
- Audio indicators: No changes
- Connection status: Update text ("Publisher: Connected", "Consumer: Connected")
- Statistics: Show single connection stats instead of per-peer

**Optional Enhancements:**
- Show SFU connection quality (RTT to server)
- Display server-side statistics (bandwidth usage)
- Add "Publishing" / "Consuming" status indicators

### Backward Compatibility

**Strategy: Hard cutover (no backward compatibility)**

**Rationale:**
- P2P and SFU are fundamentally different topologies
- Maintaining both protocols adds significant complexity
- One-time migration with clear communication to users
- Breaking change is acceptable for early-stage project

**Migration Path:**
1. Deploy new SFU backend
2. Deploy new frontend with updated signaling
3. Notify users to refresh page
4. Old clients will fail to connect (graceful error message)

**Alternative (if backward compatibility required):**
- Protocol version negotiation during handshake
- Server supports both P2P relay and SFU modes
- Client detects protocol version from server
- Significantly more complex (not recommended for one-person team)

---

## Risk Assessment & Mitigation

### Technical Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **webrtc-rs instability** | High | Medium | Pin to stable version, extensive testing, upstream contributions |
| **Increased latency** | High | Medium | Optimize forwarding path, measure at each phase, profile hotspots |
| **Server CPU bottleneck** | High | Low | Profile early, use async I/O, consider C bindings for hot paths |
| **Memory leaks** | Medium | Low | Strict cleanup on disconnect, memory profiling, integration tests |
| **NAT traversal issues** | Medium | Low | Use proven STUN/TURN servers, test on various networks |
| **Codec incompatibility** | Medium | Low | Test with multiple browsers, force Opus in SDP negotiation |

### Implementation Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **Underestimated complexity** | High | Medium | Start with Phase 1 MVP, get early feedback, adjust roadmap |
| **Scope creep** | Medium | High | Stick to phase deliverables, defer nice-to-haves to Phase 4 |
| **Integration bugs** | Medium | High | Incremental development, automated tests, manual QA per phase |
| **One-person bandwidth** | High | Low | Focus on core features only, leverage existing libraries |

### Operational Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **Server downtime** | High | Medium | Health checks, graceful restart, client reconnection logic |
| **Scaling issues** | Medium | Low | Phase 4 focuses on scalability, load testing before production |
| **Security vulnerabilities** | High | Low | Follow security best practices, regular audits, TLS/WSS only |

---

## Success Criteria

### Phase 1 Success Criteria

**Functional:**
- ✅ 5 participants can join a room
- ✅ Audio from each participant is heard by all others
- ✅ No audio dropouts or glitches under normal conditions
- ✅ Graceful handling of participant join/leave

**Performance:**
- ✅ End-to-end latency ≤ 100ms (p95)
- ✅ Server CPU usage < 20% with 5 participants
- ✅ Server memory usage < 500MB with 5 participants
- ✅ Audio quality equivalent to current P2P implementation

**Reliability:**
- ✅ No crashes in 1-hour test session
- ✅ Clean reconnection after network interruption
- ✅ All resources freed on participant disconnect

### Phase 2 Success Criteria

**Functional:**
- ✅ Automatic bitrate adaptation based on bandwidth
- ✅ Graceful degradation on poor networks (2G/3G)
- ✅ RTCP statistics visible in logs/metrics

**Performance:**
- ✅ No audio dropouts with 20% packet loss
- ✅ Recovery within 2 seconds after network restoration
- ✅ Bandwidth adaptation responds within 5 seconds

**Quality:**
- ✅ Subjective audio quality rated "good" on 3G network
- ✅ Objective MOS score > 3.5 on lossy networks

### Phase 3 Success Criteria

**Functional:**
- ✅ Simulcast with 3 quality layers
- ✅ Automatic layer selection based on consumer bandwidth
- ✅ Smooth layer switching without glitches

**Performance:**
- ✅ Support 20+ participants in a room
- ✅ 50% bandwidth savings on mobile clients
- ✅ Layer switch time < 100ms

**Scalability:**
- ✅ Linear bandwidth scaling on server (O(N))
- ✅ Constant bandwidth on clients (O(1))

### Phase 4 Success Criteria

**Operational:**
- ✅ 99.9% uptime over 1 week
- ✅ Full observability (metrics, logs, traces)
- ✅ Automated deployment pipeline
- ✅ Runbook for common issues

**Performance:**
- ✅ Support 50+ concurrent participants
- ✅ p99 latency < 50ms under load
- ✅ CPU usage < 60% at full capacity
- ✅ Automatic recovery from crashes

**Documentation:**
- ✅ Architecture documentation complete
- ✅ API documentation for all public interfaces
- ✅ Deployment guide with troubleshooting
- ✅ Performance tuning guide

---

## Next Steps

1. **Review and approve this plan** with stakeholders
2. **Create detailed implementation roadmap** (see `sfu-implementation-roadmap.md`)
3. **Design signaling protocol** in detail (see `sfu-signaling-protocol.md`)
4. **Set up development environment** (Rust toolchain, testing infrastructure)
5. **Begin Phase 1 implementation** starting with SFU router core

---

## References

- [webrtc-rs GitHub Repository](https://github.com/webrtc-rs/webrtc)
- [WebRTC Specification (W3C)](https://www.w3.org/TR/webrtc/)
- [SFU Architecture Patterns](https://bloggeek.me/webrtc-multiparty-video-alternatives/)
- [Opus Codec Documentation](https://opus-codec.org/docs/)
- Current codebase: `backend/src/main.rs`, `frontend/src/main.rs`

---

**Document Version History:**
- v1.0 (2026-01-08): Initial technical plan

# SFU Implementation Roadmap

**Version:** 1.0  
**Date:** 2026-01-08  
**Team Size:** 1 developer  
**Total Estimated Duration:** 9-13 weeks

---

## Table of Contents

1. [Overview](#overview)
2. [Phase 1: Minimal Viable SFU](#phase-1-minimal-viable-sfu)
3. [Phase 2: Bandwidth Adaptation](#phase-2-bandwidth-adaptation)
4. [Phase 3: Simulcast Support](#phase-3-simulcast-support)
5. [Phase 4: Production Optimizations](#phase-4-production-optimizations)
6. [Testing Strategy](#testing-strategy)
7. [Deployment Strategy](#deployment-strategy)
8. [Dependencies & Prerequisites](#dependencies--prerequisites)

---

## Overview

### Roadmap Structure

Each phase is broken down into:
- **Tasks:** Concrete, actionable items
- **Dependencies:** Prerequisites and blockers
- **Testing:** Validation approach
- **Deliverables:** What gets shipped
- **Success Metrics:** How we know it's done

### Time Estimates Philosophy

**Note:** Time estimates are provided for planning purposes but should be adjusted based on:
- Actual complexity discovered during implementation
- Debugging and troubleshooting time
- Learning curve with webrtc-rs
- Integration challenges

**Format:** Tasks show ranges (e.g., 2-3 days) to account for uncertainty.

### Guiding Principles

1. **Iterate quickly:** Get something working, then improve
2. **Test continuously:** Don't accumulate technical debt
3. **Document as you go:** Future you will thank present you
4. **Measure everything:** Latency, bandwidth, CPU, memory
5. **Focus on core features:** Defer nice-to-haves

---

## Phase 1: Minimal Viable SFU

**Goal:** Replace P2P mesh with basic SFU audio forwarding  
**Duration:** 2-3 weeks  
**Complexity:** High (new architecture, webrtc-rs learning curve)

### Task Breakdown

#### 1.1 Project Setup & Dependencies (2-3 days)

**Tasks:**
- [ ] Add webrtc-rs dependencies to backend/Cargo.toml
- [ ] Create new module structure: sfu/, media/, stats/
- [ ] Set up development environment with logging
- [ ] Create basic integration test framework
- [ ] Document development setup in README

**Dependencies:** None (starting point)

**Deliverables:**
- Updated Cargo.toml with all dependencies
- Clean module structure
- Working dev environment

**Validation:**
```bash
cargo build --release
cargo test
```

---

#### 1.2 WebRTC API Configuration (1-2 days)

**Tasks:**
- [ ] Implement `create_webrtc_api()` function
- [ ] Configure MediaEngine for Opus codec
- [ ] Register default interceptors
- [ ] Create ICE server configuration
- [ ] Write unit tests for API creation

**Dependencies:** 1.1 (project setup)

**Implementation Location:** `backend/src/sfu/mod.rs`

**Key Code:**
```rust
pub async fn create_webrtc_api() -> Result<API> {
    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs()?;
    
    let mut interceptor_registry = Registry::new();
    register_default_interceptors(&mut media_engine, &mut interceptor_registry)?;
    
    Ok(APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(interceptor_registry)
        .build())
}
```

**Validation:**
- API creation succeeds without errors
- MediaEngine has Opus codec registered
- Interceptors are properly configured

---

#### 1.3 Publisher Implementation (3-4 days)

**Tasks:**
- [ ] Implement Publisher struct with state management
- [ ] Create PeerConnection for receiving client media
- [ ] Implement SDP offer/answer handling
- [ ] Add ICE candidate management
- [ ] Implement ontrack handler for incoming audio
- [ ] Add connection state monitoring
- [ ] Write unit tests for Publisher lifecycle

**Dependencies:** 1.2 (WebRTC API)

**Implementation Location:** `backend/src/sfu/publisher.rs`

**Key Components:**
```rust
pub struct Publisher {
    user_id: String,
    room_id: String,
    peer_connection: Arc<RTCPeerConnection>,
    tracks: Arc<RwLock<Vec<Arc<TrackRemote>>>>,
    state: Arc<RwLock<PublisherState>>,
}

impl Publisher {
    pub async fn handle_offer(&self, sdp: String) -> Result<String>;
    pub async fn add_ice_candidate(&self, candidate: String) -> Result<()>;
}
```

**Validation:**
- Publisher can handle SDP offer and return answer
- ICE candidates are processed correctly
- Tracks are captured from ontrack event
- State transitions work correctly

---

#### 1.4 Consumer Implementation (3-4 days)

**Tasks:**
- [ ] Implement Consumer struct with state management
- [ ] Create PeerConnection for sending media to client
- [ ] Implement SDP offer generation
- [ ] Add track addition logic (addTrack)
- [ ] Implement ICE candidate management
- [ ] Add connection state monitoring
- [ ] Write unit tests for Consumer lifecycle

**Dependencies:** 1.2 (WebRTC API)

**Implementation Location:** `backend/src/sfu/consumer.rs`

**Key Components:**
```rust
pub struct Consumer {
    user_id: String,
    subscribed_to: String,
    peer_connection: Arc<RTCPeerConnection>,
    senders: Arc<RwLock<Vec<Arc<RTCRtpSender>>>>,
    state: Arc<RwLock<ConsumerState>>,
}

impl Consumer {
    pub async fn add_track(&self, track: Arc<TrackLocal>) -> Result<()>;
    pub async fn create_offer(&self) -> Result<String>;
}
```

**Validation:**
- Consumer can create SDP offer with tracks
- Tracks are added successfully via addTrack
- SDP answer is processed correctly
- State transitions work correctly

---

#### 1.5 Track Forwarding Logic (2-3 days)

**Tasks:**
- [ ] Implement TrackLocal wrapper for forwarding
- [ ] Create RTP packet forwarding loop
- [ ] Add track lifecycle management
- [ ] Implement selective forwarding (don't send to self)
- [ ] Add error handling and cleanup
- [ ] Write integration tests for forwarding

**Dependencies:** 1.3 (Publisher), 1.4 (Consumer)

**Implementation Location:** `backend/src/sfu/track_manager.rs`

**Key Logic:**
```rust
pub async fn forward_track(
    publisher_track: Arc<TrackRemote>,
    consumer_track: Arc<dyn TrackLocal + Send + Sync>,
) -> Result<()> {
    loop {
        let (rtp_packet, _) = publisher_track.read_rtp().await?;
        consumer_track.write_rtp(&rtp_packet).await?;
    }
}
```

**Validation:**
- RTP packets flow from publisher to consumer
- Forwarding doesn't introduce significant latency (<10ms)
- No memory leaks over extended runs
- Graceful shutdown on connection close

---

#### 1.6 SFU Router Integration (2-3 days)

**Tasks:**
- [ ] Implement SfuRouter struct
- [ ] Add room management logic
- [ ] Integrate Publisher and Consumer
- [ ] Implement participant tracking
- [ ] Add cleanup on disconnect
- [ ] Write integration tests

**Dependencies:** 1.3, 1.4, 1.5

**Implementation Location:** `backend/src/sfu/router.rs`

**Key Components:**
```rust
pub struct SfuRouter {
    api: Arc<API>,
    rooms: Arc<RwLock<HashMap<String, Room>>>,
    publishers: Arc<RwLock<HashMap<String, Publisher>>>,
    consumers: Arc<RwLock<HashMap<String, Vec<Consumer>>>>,
}

impl SfuRouter {
    pub async fn create_publisher(&self, user_id: String, room_id: String) -> Result<Publisher>;
    pub async fn create_consumer(&self, user_id: String, room_id: String) -> Result<Consumer>;
    pub async fn forward_track(&self, publisher_id: String, track: Arc<TrackRemote>) -> Result<()>;
}
```

**Validation:**
- Multiple publishers can coexist in same room
- Consumers receive tracks from all publishers
- Room cleanup works on last participant leaving
- No resource leaks

---

#### 1.7 Signaling Protocol Update (2 days)

**Tasks:**
- [ ] Add new message types to ClientMessage enum
- [ ] Add new message types to ServerMessage enum
- [ ] Update WebSocket handler to route messages
- [ ] Implement PublishOffer/Answer handlers
- [ ] Implement Subscribe/ConsumeOffer handlers
- [ ] Add ICE candidate routing
- [ ] Write protocol tests

**Dependencies:** 1.6 (SFU Router)

**Implementation Location:** `backend/src/signaling/messages.rs`, `backend/src/signaling/handler.rs`

**New Message Types:**
- PublishOffer, PublishAnswer, PublishIceCandidate
- Subscribe, ConsumeOffer, ConsumeAnswer, ConsumeIceCandidate
- TrackAdded, TrackRemoved

**Validation:**
- All new message types serialize/deserialize correctly
- WebSocket routes messages to correct handlers
- End-to-end flow works (register → join → publish → consume)

---

#### 1.8 Frontend Publisher Implementation (1-2 days)

**Tasks:**
- [ ] Create `setup_publisher()` function
- [ ] Update microphone request to trigger publishing
- [ ] Implement PublishOffer sending
- [ ] Handle PublishAnswer from server
- [ ] Add ICE candidate exchange for publisher
- [ ] Remove old P2P peer connection logic
- [ ] Test with local development server

**Dependencies:** 1.7 (Signaling protocol)

**Implementation Location:** `frontend/src/main.rs`

**Key Changes:**
```rust
// Replace peer_connections HashMap with single connections
let publisher_connection = use_signal(|| None::<RtcPeerConnection>);
let consumer_connection = use_signal(|| None::<RtcPeerConnection>);

async fn setup_publisher(stream: MediaStream) -> Result<()> {
    let pc = create_rtc_peer_connection()?;
    for track in stream.get_tracks() {
        pc.add_track(&track, &stream)?;
    }
    let offer = pc.create_offer().await?;
    pc.set_local_description(&offer).await?;
    send_message(ClientMessage::PublishOffer { sdp: offer.sdp })?;
    Ok(())
}
```

**Validation:**
- Publisher connection established successfully
- Audio track added to connection
- SDP offer sent to server
- Answer received and applied

---

#### 1.9 Frontend Consumer Implementation (2-3 days)

**Tasks:**
- [ ] Create `setup_consumer()` function
- [ ] Implement Subscribe message sending
- [ ] Handle ConsumeOffer from server
- [ ] Send ConsumeAnswer back to server
- [ ] Add ICE candidate exchange for consumer
- [ ] Update ontrack handler for multiple streams
- [ ] Map tracks to users via stream IDs
- [ ] Test with multiple participants

**Dependencies:** 1.8 (Frontend Publisher)

**Implementation Location:** `frontend/src/main.rs`

**Key Changes:**
```rust
async fn setup_consumer() -> Result<()> {
    let pc = create_rtc_peer_connection()?;
    
    pc.set_ontrack(move |event: RtcTrackEvent| {
        let track = event.track();
        let streams = event.streams();
        
        if let Some(stream) = streams.get(0).dyn_into::<MediaStream>().ok() {
            let stream_id = stream.id();
            let user_id = extract_user_id(&stream_id)?;
            play_remote_audio(track, user_id);
        }
    });
    
    send_message(ClientMessage::Subscribe { room_id })?;
    Ok(())
}
```

**Validation:**
- Consumer connection established successfully
- ConsumeOffer received with track mappings
- Multiple tracks received and played
- Audio from each participant is audible

---

#### 1.10 End-to-End Testing & Bug Fixes (2-3 days)

**Tasks:**
- [ ] Test with 2 participants (basic case)
- [ ] Test with 5 participants (target capacity)
- [ ] Measure end-to-end latency
- [ ] Test join/leave scenarios
- [ ] Test reconnection after network interruption
- [ ] Fix bugs discovered during testing
- [ ] Document known issues

**Dependencies:** 1.9 (Frontend Consumer)

**Test Scenarios:**
1. **Basic Flow:** A joins, B joins, both hear each other
2. **Late Join:** A and B talking, C joins, hears both
3. **Leave:** A, B, C talking, B leaves, A and C still hear each other
4. **Reconnect:** A loses network, reconnects, resumes audio
5. **Rapid Join/Leave:** Stress test with quick participant changes

**Metrics to Collect:**
- End-to-end latency (glass-to-glass)
- Server CPU usage (per participant)
- Server memory usage
- Bandwidth usage (per participant)
- Connection establishment time

**Validation:**
- All test scenarios pass
- Latency < 100ms (p95)
- No audio dropouts under normal conditions
- Clean resource cleanup on disconnect

---

### Phase 1 Summary

**Total Duration:** 2-3 weeks

**Task Dependencies Graph:**
```
1.1 (Setup)
  └─► 1.2 (WebRTC API)
       ├─► 1.3 (Publisher)
       │    └─┐
       └─► 1.4 (Consumer)
            └─┬─► 1.5 (Track Forwarding)
              │
              └─► 1.6 (SFU Router)
                   └─► 1.7 (Signaling)
                        ├─► 1.8 (Frontend Publisher)
                        │    └─► 1.9 (Frontend Consumer)
                        │         └─► 1.10 (E2E Testing)
```

**Critical Path:** 1.1 → 1.2 → 1.3 → 1.5 → 1.6 → 1.7 → 1.8 → 1.9 → 1.10

**Deliverables:**
- ✅ Working SFU server with audio forwarding
- ✅ Updated frontend with publisher/consumer model
- ✅ Basic signaling protocol implemented
- ✅ Tested with 5 participants
- ✅ Documentation updated

---

## Phase 2: Bandwidth Adaptation

**Goal:** Add intelligent bandwidth management for varying network conditions  
**Duration:** 2 weeks  
**Complexity:** Medium (requires RTCP understanding)

### Task Breakdown

#### 2.1 RTCP Statistics Collection (2-3 days)

**Tasks:**
- [ ] Implement RTCP packet parsing
- [ ] Extract sender/receiver reports
- [ ] Calculate packet loss percentage
- [ ] Measure RTT from RTCP feedback
- [ ] Track jitter from RTP timestamps
- [ ] Store statistics per connection
- [ ] Add statistics API endpoint

**Dependencies:** Phase 1 complete

**Implementation Location:** `backend/src/stats/rtcp_processor.rs`

**Key Metrics:**
```rust
pub struct RtcpStats {
    packets_sent: u64,
    packets_lost: u64,
    bytes_sent: u64,
    jitter_ms: f64,
    rtt_ms: f64,
    timestamp: Instant,
}
```

**Validation:**
- RTCP packets are parsed correctly
- Statistics match browser-reported values
- Metrics update every 1-2 seconds

---

#### 2.2 Bandwidth Estimator (2-3 days)

**Tasks:**
- [ ] Implement REMB (Receiver Estimated Maximum Bitrate)
- [ ] Implement Transport-CC (Transport-wide Congestion Control)
- [ ] Create bandwidth estimation algorithm
- [ ] Add smoothing and filtering logic
- [ ] Implement adjustment thresholds
- [ ] Write unit tests for estimator

**Dependencies:** 2.1 (RTCP Statistics)

**Implementation Location:** `backend/src/media/bandwidth.rs`

**Key Algorithm:**
```rust
pub struct BandwidthEstimator {
    current_estimate_kbps: f64,
    target_bitrate_kbps: f64,
    packet_loss_threshold: f64,
}

impl BandwidthEstimator {
    pub fn update(&mut self, stats: &RtcpStats) {
        if stats.packet_loss > self.packet_loss_threshold {
            // Reduce bitrate
            self.target_bitrate_kbps *= 0.85;
        } else {
            // Gradually increase
            self.target_bitrate_kbps *= 1.05;
        }
        // Clamp to reasonable range
        self.target_bitrate_kbps = self.target_bitrate_kbps.clamp(16.0, 128.0);
    }
}
```

**Validation:**
- Bandwidth estimate responds to packet loss
- Estimate doesn't oscillate wildly
- Converges to stable value under steady conditions

---

#### 2.3 Quality Adaptation Logic (2 days)

**Tasks:**
- [ ] Implement bitrate signaling to publishers
- [ ] Add quality level selection logic
- [ ] Implement graceful degradation strategy
- [ ] Add hysteresis to prevent oscillation
- [ ] Send QualityUpdate messages to clients
- [ ] Write integration tests

**Dependencies:** 2.2 (Bandwidth Estimator)

**Implementation Location:** `backend/src/media/quality_adapter.rs`

**Strategy:**
```
Quality Levels:
- High: 64 kbps (full quality Opus)
- Medium: 32 kbps (reduced quality)
- Low: 16 kbps (minimum viable)

Adaptation Rules:
- Packet loss > 10% → drop to next lower level
- Packet loss < 2% for 10s → try next higher level
- Never change more than once per 5 seconds
```

**Validation:**
- Quality adapts to simulated packet loss
- Audio remains intelligible at low quality
- No frequent quality switches

---

#### 2.4 Frontend Bandwidth Reporting (1 day)

**Tasks:**
- [ ] Implement UpdateBandwidth message sending
- [ ] Monitor client-side connection stats
- [ ] Report available bandwidth to server
- [ ] Display quality indicator in UI
- [ ] Test with network throttling

**Dependencies:** 2.3 (Quality Adaptation)

**Implementation Location:** `frontend/src/main.rs`

**Validation:**
- Client reports bandwidth accurately
- UI shows quality level
- Quality changes visible to user

---

#### 2.5 Testing with Network Simulation (2-3 days)

**Tasks:**
- [ ] Set up network simulation tools (tc, netem)
- [ ] Test with 5% packet loss
- [ ] Test with 20% packet loss
- [ ] Test with variable latency (50-200ms)
- [ ] Test with bandwidth limits (1Mbps, 512kbps, 256kbps)
- [ ] Measure subjective audio quality (MOS)
- [ ] Document adaptation behavior

**Dependencies:** 2.4 (Frontend Bandwidth Reporting)

**Test Scenarios:**
1. **Good network:** <1% loss, <50ms latency
2. **3G network:** 3% loss, 100ms latency, 1Mbps bandwidth
3. **2G network:** 10% loss, 200ms latency, 256kbps bandwidth
4. **Recovery:** Start poor, improve to good

**Metrics:**
- Audio quality rating (1-5 scale)
- Dropout count per minute
- Time to adapt to new conditions
- Subjective listening tests

**Validation:**
- No audio dropouts with <20% packet loss
- Quality degrades gracefully
- Recovery is quick (<5 seconds)

---

### Phase 2 Summary

**Total Duration:** 2 weeks

**Task Dependencies:**
```
2.1 (RTCP Stats)
  └─► 2.2 (Bandwidth Estimator)
       └─► 2.3 (Quality Adaptation)
            └─► 2.4 (Frontend Reporting)
                 └─► 2.5 (Network Testing)
```

**Deliverables:**
- ✅ RTCP statistics collection
- ✅ Bandwidth adaptation algorithm
- ✅ Quality control system
- ✅ Tested on poor networks
- ✅ Updated UI with quality indicators

---

## Phase 3: Simulcast Support

**Goal:** Enable multiple quality layers for bandwidth efficiency  
**Duration:** 2-3 weeks  
**Complexity:** High (complex SDP negotiation)

### Task Breakdown

#### 3.1 SDP Simulcast Negotiation (3-4 days)

**Tasks:**
- [ ] Research Opus simulcast encoding
- [ ] Implement SDP munging for simulcast
- [ ] Add rid (restriction identifier) parameters
- [ ] Update offer/answer generation
- [ ] Handle multiple encodings in SDP
- [ ] Test SDP parsing and generation
- [ ] Document simulcast SDP format

**Dependencies:** Phase 2 complete

**Implementation Location:** `backend/src/media/codec.rs`

**Key SDP Changes:**
```
m=audio 9 UDP/TLS/RTP/SAVPF 111
a=rtpmap:111 opus/48000/2
a=rid:h send
a=rid:m send
a=rid:l send
a=simulcast:send h;m;l
```

**Validation:**
- SDP with simulcast parses correctly
- Multiple rids are negotiated
- Browser accepts simulcast SDP

---

#### 3.2 Layer Selection Algorithm (2-3 days)

**Tasks:**
- [ ] Implement layer selection based on bandwidth
- [ ] Add layer switching logic
- [ ] Prevent rapid layer changes (hysteresis)
- [ ] Track active layer per consumer
- [ ] Optimize for minimal switching overhead
- [ ] Write unit tests for selector

**Dependencies:** 3.1 (SDP Negotiation)

**Implementation Location:** `backend/src/media/layer_selector.rs`

**Algorithm:**
```rust
pub struct LayerSelector {
    available_bandwidth_kbps: f64,
    current_layer: SimulcastLayer,
}

pub enum SimulcastLayer {
    High,   // 64 kbps
    Medium, // 32 kbps
    Low,    // 16 kbps
}

impl LayerSelector {
    pub fn select_layer(&mut self) -> SimulcastLayer {
        match self.available_bandwidth_kbps {
            bw if bw >= 64.0 => SimulcastLayer::High,
            bw if bw >= 32.0 => SimulcastLayer::Medium,
            _ => SimulcastLayer::Low,
        }
    }
}
```

**Validation:**
- Layer selection is deterministic
- Switching has hysteresis (no oscillation)
- Correct layer is forwarded to consumer

---

#### 3.3 Multi-Layer Forwarding (2-3 days)

**Tasks:**
- [ ] Extend track forwarding for multiple layers
- [ ] Implement layer filtering in RTP forwarding
- [ ] Add layer metadata to track mappings
- [ ] Handle layer switching without glitches
- [ ] Optimize memory usage for multi-layer buffering
- [ ] Write integration tests

**Dependencies:** 3.2 (Layer Selection)

**Implementation Location:** `backend/src/sfu/track_manager.rs`

**Key Logic:**
```rust
pub struct MultiLayerForwarder {
    publisher_tracks: HashMap<SimulcastLayer, Arc<TrackRemote>>,
    consumer_track: Arc<dyn TrackLocal + Send + Sync>,
    active_layer: Arc<RwLock<SimulcastLayer>>,
}

impl MultiLayerForwarder {
    pub async fn forward_with_layer_selection(&self) -> Result<()> {
        loop {
            let layer = *self.active_layer.read().await;
            let track = self.publisher_tracks.get(&layer)?;
            
            let (rtp_packet, _) = track.read_rtp().await?;
            self.consumer_track.write_rtp(&rtp_packet).await?;
        }
    }
}
```

**Validation:**
- All layers are forwarded correctly
- Layer switches don't cause audio glitches
- No memory leaks with multi-layer buffering

---

#### 3.4 Frontend Simulcast Publishing (2 days)

**Tasks:**
- [ ] Configure RTCRtpSender with simulcast parameters
- [ ] Update offer generation for simulcast
- [ ] Test multi-layer encoding in browser
- [ ] Verify bandwidth savings
- [ ] Test on mobile devices
- [ ] Document browser compatibility

**Dependencies:** 3.3 (Multi-Layer Forwarding)

**Implementation Location:** `frontend/src/main.rs`

**Key Changes:**
```rust
// Add simulcast to sender parameters
let sender = pc.add_track(&track, &stream)?;
let params = sender.get_parameters();

// Configure encodings for simulcast
params.encodings = vec![
    { rid: "h", max_bitrate: 64000 },
    { rid: "m", max_bitrate: 32000, scale_resolution_down_by: 1.0 },
    { rid: "l", max_bitrate: 16000, scale_resolution_down_by: 1.0 },
];

sender.set_parameters(params)?;
```

**Validation:**
- Browser sends multiple layers
- Server receives all layers
- Bitrates match expected values

---

#### 3.5 Scale Testing (2-3 days)

**Tasks:**
- [ ] Test with 10 participants
- [ ] Test with 20 participants
- [ ] Test with 30+ participants (stretch goal)
- [ ] Measure server resource usage at scale
- [ ] Identify bottlenecks
- [ ] Optimize hot paths
- [ ] Document scaling limits

**Dependencies:** 3.4 (Frontend Simulcast)

**Test Setup:**
- Simulate multiple clients from single machine
- Use automated browser instances (Puppeteer)
- Monitor server metrics continuously

**Metrics:**
- Server CPU usage (% per participant)
- Server memory usage (MB per participant)
- Network bandwidth (kbps per participant)
- Latency at different scales

**Validation:**
- 20+ participants with acceptable quality
- Server CPU < 60% at capacity
- Latency remains < 100ms

---

### Phase 3 Summary

**Total Duration:** 2-3 weeks

**Task Dependencies:**
```
3.1 (SDP Simulcast)
  └─► 3.2 (Layer Selection)
       └─► 3.3 (Multi-Layer Forwarding)
            └─► 3.4 (Frontend Simulcast)
                 └─► 3.5 (Scale Testing)
```

**Deliverables:**
- ✅ Simulcast SDP negotiation
- ✅ Layer selection algorithm
- ✅ Multi-layer forwarding
- ✅ Frontend simulcast support
- ✅ Scale testing results (20+ participants)

---

## Phase 4: Production Optimizations

**Goal:** Harden system for production deployment  
**Duration:** 2-3 weeks  
**Complexity:** Medium (operations focus)

### Task Breakdown

#### 4.1 Metrics & Monitoring (2-3 days)

**Tasks:**
- [ ] Add Prometheus metrics export
- [ ] Instrument key code paths
- [ ] Create Grafana dashboards
- [ ] Set up alerting rules
- [ ] Document metrics and alerts
- [ ] Test metrics under load

**Dependencies:** Phase 3 complete

**Implementation Location:** `backend/src/stats/collector.rs`

**Key Metrics:**
```
# Server metrics
sfu_active_publishers_total
sfu_active_consumers_total
sfu_rooms_active_total
sfu_rtp_packets_forwarded_total
sfu_connection_errors_total

# Performance metrics
sfu_rtp_forwarding_latency_ms
sfu_peer_connection_duration_seconds
sfu_cpu_usage_percent
sfu_memory_usage_bytes
```

**Validation:**
- Metrics are accurate
- Dashboards show real-time data
- Alerts fire correctly

---

#### 4.2 Structured Logging (1 day)

**Tasks:**
- [ ] Add structured logging with tracing
- [ ] Define log levels appropriately
- [ ] Add correlation IDs for requests
- [ ] Configure log rotation
- [ ] Set up log aggregation (optional)
- [ ] Document logging strategy

**Dependencies:** None (can parallelize)

**Implementation Location:** Throughout codebase

**Log Levels:**
- ERROR: Connection failures, critical bugs
- WARN: Degraded performance, retries
- INFO: Connection events, room events
- DEBUG: Detailed SFU operations
- TRACE: RTP packet-level debugging

**Validation:**
- Logs are structured and searchable
- Correlation IDs work across components
- No excessive logging (performance impact)

---

#### 4.3 Error Recovery & Fault Tolerance (2-3 days)

**Tasks:**
- [ ] Implement graceful degradation strategies
- [ ] Add automatic reconnection logic
- [ ] Handle partial failures (some consumers fail)
- [ ] Implement circuit breaker pattern
- [ ] Add health check endpoint
- [ ] Test failure scenarios
- [ ] Document recovery procedures

**Dependencies:** 4.1, 4.2

**Implementation Location:** `backend/src/sfu/router.rs`

**Failure Scenarios:**
1. Publisher disconnects → notify consumers, keep room alive
2. Consumer disconnects → cleanup, don't affect others
3. Server restarts → clients reconnect automatically
4. Memory exhaustion → reject new connections, keep existing
5. Network partition → detect and cleanup stale connections

**Validation:**
- All failure scenarios handled gracefully
- No resource leaks after failures
- Automatic recovery works

---

#### 4.4 Security Hardening (2 days)

**Tasks:**
- [ ] Enable TLS/WSS support
- [ ] Add rate limiting for WebSocket messages
- [ ] Implement DoS protection
- [ ] Validate all client inputs
- [ ] Add authentication tokens (optional)
- [ ] Security audit and fixes
- [ ] Document security considerations

**Dependencies:** None (can parallelize)

**Security Measures:**
```rust
// Rate limiting
const MAX_MESSAGES_PER_SECOND: u32 = 100;
const MAX_ROOMS_PER_USER: u32 = 5;

// Input validation
fn validate_room_id(room_id: &str) -> Result<()> {
    if room_id.len() > 100 || !room_id.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(Error::InvalidRoomId);
    }
    Ok(())
}
```

**Validation:**
- Rate limiting prevents abuse
- Invalid inputs are rejected
- WSS works correctly

---

#### 4.5 Performance Optimization (2-3 days)

**Tasks:**
- [ ] Profile CPU hotspots with perf/flamegraph
- [ ] Optimize RTP forwarding loop
- [ ] Consider lock-free data structures (DashMap)
- [ ] Reduce allocations in hot paths
- [ ] Optimize SDP parsing
- [ ] Benchmark before/after optimizations
- [ ] Document performance characteristics

**Dependencies:** 4.1 (Metrics for profiling)

**Optimization Targets:**
- RTP forwarding latency < 5ms
- Lock contention < 1% CPU time
- Memory allocations in hot path minimized
- SDP parsing < 1ms

**Validation:**
- Profiling shows improvements
- Benchmarks confirm gains
- No functional regressions

---

#### 4.6 Docker & Deployment (2 days)

**Tasks:**
- [ ] Create Dockerfile for backend
- [ ] Create docker-compose.yml for local dev
- [ ] Set up CI/CD pipeline (GitHub Actions)
- [ ] Create deployment scripts
- [ ] Document deployment process
- [ ] Test deployment on staging environment
- [ ] Write operations runbook

**Dependencies:** 4.3, 4.4, 4.5

**Implementation Location:** `Dockerfile`, `.github/workflows/`

**Dockerfile:**
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/backend /usr/local/bin/
EXPOSE 8080
CMD ["backend"]
```

**Validation:**
- Docker image builds successfully
- Container runs without errors
- Can deploy to production environment

---

#### 4.7 Load Testing & Capacity Planning (2-3 days)

**Tasks:**
- [ ] Set up load testing framework
- [ ] Run load tests with various participant counts
- [ ] Measure server capacity limits
- [ ] Identify bottlenecks
- [ ] Document capacity recommendations
- [ ] Create capacity planning guide
- [ ] Test horizontal scaling (optional)

**Dependencies:** 4.6 (Deployment)

**Load Test Scenarios:**
1. Ramp up: 0 → 50 participants over 10 minutes
2. Sustained: 50 participants for 1 hour
3. Spike: Sudden jump 10 → 40 participants
4. Churn: Participants constantly joining/leaving

**Metrics:**
- Maximum participants per server instance
- CPU/memory usage curves
- Latency under load
- Connection establishment time

**Validation:**
- Server handles target load (50+ participants)
- Performance meets SLAs
- Capacity recommendations documented

---

#### 4.8 Documentation & Handoff (2 days)

**Tasks:**
- [ ] Update architecture documentation
- [ ] Document all APIs and protocols
- [ ] Write deployment guide
- [ ] Create troubleshooting guide
- [ ] Document monitoring and alerts
- [ ] Write performance tuning guide
- [ ] Create user-facing documentation

**Dependencies:** All previous tasks

**Documentation Structure:**
```
docs/
├── architecture.md
├── api-reference.md
├── deployment.md
├── monitoring.md
├── troubleshooting.md
├── performance-tuning.md
└── operations-runbook.md
```

**Validation:**
- Documentation is complete and accurate
- New team member can deploy using docs
- Troubleshooting guide covers common issues

---

### Phase 4 Summary

**Total Duration:** 2-3 weeks

**Task Dependencies:**
```
4.1 (Metrics) ──┐
                ├─► 4.5 (Performance)
4.2 (Logging) ──┤    │
                ├────┴─► 4.3 (Error Recovery)
4.4 (Security) ─┤              │
                └──────────────┴─► 4.6 (Deployment)
                                    └─► 4.7 (Load Testing)
                                         └─► 4.8 (Documentation)
```

**Deliverables:**
- ✅ Full observability stack
- ✅ Production-ready deployment
- ✅ Load tested and capacity planned
- ✅ Complete documentation
- ✅ Operations runbook

---

## Testing Strategy

### Unit Testing

**Scope:** Individual components in isolation

**Framework:** Rust's built-in test framework

**Coverage Goals:** >80% for core SFU logic

**Example:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_publisher_handles_offer() {
        let publisher = Publisher::new("user1", "room1", api).await.unwrap();
        let answer = publisher.handle_offer(VALID_SDP_OFFER).await;
        assert!(answer.is_ok());
    }
}
```

**Components to Unit Test:**
- Publisher lifecycle
- Consumer lifecycle
- SDP parsing and generation
- Bandwidth estimator
- Layer selector
- Message serialization/deserialization

---

### Integration Testing

**Scope:** Component interactions

**Framework:** Tokio + webrtc test utilities

**Test Environment:** In-process with mocked WebRTC transports

**Example:**
```rust
#[tokio::test]
async fn test_track_forwarding() {
    let router = SfuRouter::new().await.unwrap();
    
    // Create publisher
    let publisher = router.create_publisher("user1", "room1").await.unwrap();
    
    // Simulate track
    let track = create_mock_track();
    
    // Create consumer
    let consumer = router.create_consumer("user2", "room1").await.unwrap();
    
    // Verify forwarding
    router.forward_track("user1", track).await.unwrap();
    
    // Assert consumer receives track
    assert_eq!(consumer.get_tracks().await.len(), 1);
}
```

**Scenarios to Test:**
- Publisher → Consumer forwarding
- Multiple publishers in same room
- Late join (consumer joins after publishers)
- Early leave (publisher leaves while consumers active)
- Reconnection after disconnect

---

### End-to-End Testing

**Scope:** Full system with real browsers

**Tools:**
- Puppeteer (automated browser control)
- WebDriver (browser automation)
- Manual testing

**Test Matrix:**
| Browser | OS | Version |
|---------|-----|---------|
| Chrome | macOS | Latest |
| Chrome | Linux | Latest |
| Firefox | macOS | Latest |
| Safari | macOS | Latest |
| Chrome | Android | Latest |
| Safari | iOS | Latest |

**Test Scenarios:**
1. **Happy Path:** 2 users join, talk, leave
2. **Scale:** 10+ users in same room
3. **Network Issues:** Simulate packet loss, latency
4. **Mobile:** Test on mobile devices
5. **Stress:** Rapid join/leave cycles

**Validation:**
- Audio quality is acceptable
- No crashes or errors
- UI is responsive
- All features work

---

### Performance Testing

**Tools:**
- Custom load generator (simulated clients)
- Prometheus metrics
- Flamegraphs for profiling

**Test Cases:**
1. **Latency Test:**
   - Measure glass-to-glass latency
   - Target: <100ms p95

2. **Throughput Test:**
   - Measure max participants per server
   - Target: 50+ participants

3. **Resource Usage:**
   - Measure CPU, memory, network
   - Target: <60% CPU at capacity

4. **Endurance Test:**
   - Run for 24 hours continuously
   - Target: No memory leaks, no crashes

**Metrics to Collect:**
- RTP forwarding latency
- SDP negotiation time
- Connection establishment time
- CPU usage per participant
- Memory usage per participant
- Network bandwidth per participant

---

### Regression Testing

**Strategy:** Automated test suite run on every commit

**CI/CD Integration:**
```yaml
# .github/workflows/test.yml
name: Test
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
      - run: cargo test --all-features
      - run: cargo clippy -- -D warnings
      - run: cargo fmt -- --check
```

**Test Stages:**
1. Unit tests (fast, <1 minute)
2. Integration tests (medium, <5 minutes)
3. E2E tests (slow, <15 minutes, on PR only)

**Validation:**
- All tests pass before merge
- No new warnings or clippy errors
- Code is formatted correctly

---

## Deployment Strategy

### Development Environment

**Setup:**
```bash
# Clone repository
git clone https://github.com/user/cheenhub.git
cd cheenhub

# Install dependencies
cargo build

# Run backend
cd backend
cargo run

# Run frontend (separate terminal)
cd frontend
dx serve
```

**Configuration:**
- Backend listens on localhost:8080
- Frontend served on localhost:8000
- WebSocket at ws://localhost:8080/ws
- Hot reload enabled for development

---

### Staging Environment

**Infrastructure:**
- Single server instance
- Docker containers
- TLS certificates (Let's Encrypt)
- Basic monitoring

**Deployment:**
```bash
# Build Docker image
docker build -t sfu-backend:latest .

# Run with docker-compose
docker-compose up -d

# Check logs
docker-compose logs -f
```

**Configuration:**
- Backend at staging.example.com:443
- WSS at wss://staging.example.com/ws
- Prometheus metrics at :9090
- Grafana dashboards at :3000

**Testing on Staging:**
- Deploy new features here first
- Run full E2E test suite
- Manual QA testing
- Performance validation

---

### Production Environment

**Infrastructure Options:**

**Option 1: Single Server (MVP)**
- Cloud VPS (AWS, GCP, DigitalOcean)
- 4-8 vCPUs, 8-16GB RAM
- 100GB SSD storage
- 1Gbps network
- Cost: ~$50-100/month

**Option 2: Kubernetes (Scale)**
- Managed Kubernetes cluster
- Auto-scaling (2-10 pods)
- Load balancer
- Persistent storage
- Cost: ~$200-500/month

**Deployment Process:**

1. **Build:**
   ```bash
   docker build -t sfu-backend:v1.0.0 .
   docker push registry.example.com/sfu-backend:v1.0.0
   ```

2. **Deploy:**
   ```bash
   kubectl apply -f k8s/deployment.yaml
   kubectl rollout status deployment/sfu-backend
   ```

3. **Verify:**
   - Health check endpoint returns 200 OK
   - Metrics are being collected
   - Test with real clients

4. **Rollback (if needed):**
   ```bash
   kubectl rollout undo deployment/sfu-backend
   ```

---

### Monitoring & Alerting

**Metrics to Monitor:**
- Server health (CPU, memory, disk, network)
- SFU metrics (participants, rooms, connections)
- Performance metrics (latency, packet loss)
- Error rates (connection failures, timeouts)

**Alerting Rules:**
```yaml
groups:
  - name: sfu_alerts
    rules:
      - alert: HighCPUUsage
        expr: cpu_usage_percent > 80
        for: 5m
        annotations:
          summary: "SFU server CPU usage is high"
          
      - alert: HighErrorRate
        expr: rate(connection_errors_total[5m]) > 0.1
        for: 2m
        annotations:
          summary: "SFU connection error rate is high"
```

**Notification Channels:**
- Email for non-urgent alerts
- Slack/Discord for important alerts
- PagerDuty for critical alerts (production)

---

### Rollback Strategy

**Scenarios:**

1. **Minor Issues:** Fix forward with hotfix
2. **Major Issues:** Rollback to previous version

**Rollback Process:**
```bash
# Kubernetes
kubectl rollout undo deployment/sfu-backend

# Docker Compose
docker-compose down
docker-compose up -d --force-recreate --no-deps --build backend:v0.9.0

# Verify
curl -f https://example.com/health || echo "Rollback failed"
```

**Testing After Rollback:**
- Health check passes
- Existing connections continue working
- New connections can be established
- No data loss

---

### Maintenance Windows

**Strategy:** Rolling updates (zero downtime)

**Process:**
1. Update 50% of instances
2. Monitor for 5 minutes
3. Update remaining 50%
4. Full monitoring for 30 minutes

**Communication:**
- Announce maintenance window 24h in advance
- Status page shows current status
- Post-mortem after significant incidents

---

## Dependencies & Prerequisites

### Development Environment

**Required:**
- Rust 1.75+ (stable)
- Node.js 18+ (for Dioxus CLI)
- Git

**Recommended:**
- Docker & Docker Compose
- VS Code with rust-analyzer
- Chrome/Firefox DevTools

**Installation:**
```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Dioxus CLI
cargo install dioxus-cli

# Docker (Ubuntu)
sudo apt-get install docker.io docker-compose

# Verify
rustc --version
dx --version
docker --version
```

---

### External Services

**STUN Servers:**
- Google Public STUN: stun:stun.l.google.com:19302
- Alternative: stun:stun1.l.google.com:19302

**TURN Servers (Optional):**
- Required for strict NAT/firewall scenarios
- Options:
  - Self-hosted: coturn
  - Managed: Twilio, Xirsys
  - Cost: ~$0.005 per GB

**Monitoring Stack (Phase 4):**
- Prometheus (metrics)
- Grafana (dashboards)
- Loki (logs, optional)

---

### Knowledge Prerequisites

**Required Skills:**
- Rust programming (async/await, tokio)
- WebRTC fundamentals (SDP, ICE, RTP)
- WebSocket protocol
- Basic networking (TCP, UDP, NAT)

**Recommended Reading:**
- [WebRTC for the Curious](https://webrtcforthecurious.com/)
- [webrtc-rs documentation](https://docs.rs/webrtc/)
- [Tokio tutorial](https://tokio.rs/tokio/tutorial)

---

## Risk Mitigation

### Technical Risks

| Risk | Mitigation |
|------|------------|
| webrtc-rs API changes | Pin versions, monitor releases, budget time for upgrades |
| Performance bottlenecks | Profile early and often, optimize hot paths |
| Browser compatibility | Test on multiple browsers, use standard WebRTC APIs |
| Network issues | Implement robust error handling, retry logic |

### Schedule Risks

| Risk | Mitigation |
|------|------------|
| Underestimated complexity | Use time ranges, add buffer time, re-plan if needed |
| Scope creep | Strict phase boundaries, defer non-critical features |
| Bugs and debugging | Allocate 20-30% time for bug fixes |

---

## Timeline Summary

### Overall Timeline

```
Week 1-3:   Phase 1 (Minimal Viable SFU)
Week 4-5:   Phase 2 (Bandwidth Adaptation)
Week 6-8:   Phase 3 (Simulcast Support)
Week 9-11:  Phase 4 (Production Optimizations)
Week 12:    Buffer / Polish / Documentation
Week 13:    Production deployment
```

**Total Duration:** 9-13 weeks (2-3 months)

### Milestones

- **M1 (Week 3):** Phase 1 MVP - Basic SFU working
- **M2 (Week 5):** Phase 2 Complete - Bandwidth adaptation
- **M3 (Week 8):** Phase 3 Complete - Simulcast support
- **M4 (Week 11):** Phase 4 Complete - Production ready
- **M5 (Week 13):** Go-live

### Continuous Activities

Throughout all phases:
- Daily: Code, test, commit
- Weekly: Integration testing, progress review
- Bi-weekly: Performance measurement
- Monthly: Architecture review, documentation update

---

## Success Criteria Checklist

### Phase 1
- [ ] 5 participants can talk simultaneously
- [ ] End-to-end latency < 100ms
- [ ] No audio dropouts under normal conditions
- [ ] Clean resource cleanup on disconnect

### Phase 2
- [ ] No audio dropouts with 20% packet loss
- [ ] Bandwidth adapts within 5 seconds
- [ ] Quality degrades gracefully on poor networks
- [ ] Recovery after network restoration < 2 seconds

### Phase 3
- [ ] 20+ participants supported
- [ ] Simulcast reduces bandwidth by 50% on mobile
- [ ] Layer switches are glitch-free
- [ ] Server CPU < 60% at capacity

### Phase 4
- [ ] 99.9% uptime over 1 week test period
- [ ] Full observability stack operational
- [ ] Load tested to 50+ participants
- [ ] Complete documentation delivered
- [ ] Production deployment successful

---

**Document Version History:**
- v1.0 (2026-01-08): Initial roadmap

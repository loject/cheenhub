# SFU Signaling Protocol Specification

**Version:** 1.0  
**Date:** 2026-01-08  
**Protocol:** WebSocket + JSON

---

## Table of Contents

1. [Overview](#overview)
2. [Connection Lifecycle](#connection-lifecycle)
3. [Message Types](#message-types)
4. [Publisher Flow](#publisher-flow)
5. [Consumer Flow](#consumer-flow)
6. [Transport Management](#transport-management)
7. [State Machines](#state-machines)
8. [Error Handling](#error-handling)
9. [Message Examples](#message-examples)

---

## Overview

### Protocol Design

**Transport:** WebSocket (ws:// or wss://)  
**Encoding:** JSON with tagged enum types  
**Direction:** Bidirectional (Client ↔ Server)

**Key Principles:**
- Clear separation between publishing and consuming
- Server-initiated consumer offers (role reversal from P2P)
- Explicit track lifecycle events
- Stateless message design (can be retried safely)

### Current vs New Protocol

**Current (P2P Mesh):**
```
ClientMessage:
- Register, CreateRoom, JoinRoom, LeaveRoom
- WebrtcOffer, WebrtcAnswer, IceCandidate (peer-to-peer)

ServerMessage:
- Registered, RoomJoined, UserJoined, UserLeft
- WebrtcOffer, WebrtcAnswer, IceCandidate (relayed)
```

**New (SFU):**
```
ClientMessage:
- Register, CreateRoom, JoinRoom, LeaveRoom (unchanged)
+ PublishOffer, PublishAnswer, PublishIceCandidate
+ ConsumeAnswer, ConsumeIceCandidate
+ Subscribe, Unsubscribe

ServerMessage:
- Registered, RoomJoined, UserJoined, UserLeft (unchanged)
+ PublishAnswer, PublishIceCandidate
+ ConsumeOffer, ConsumeIceCandidate
+ TrackAdded, TrackRemoved
+ QualityUpdate (Phase 2)
```

---

## Connection Lifecycle

### Connection States

```
┌──────────────┐
│ Disconnected │
└──────┬───────┘
       │ WebSocket connect
       ▼
┌──────────────┐
│  Connected   │
└──────┬───────┘
       │ Register message
       ▼
┌──────────────┐
│ Registered   │ (has user_id)
└──────┬───────┘
       │ JoinRoom message
       ▼
┌──────────────┐
│   In Room    │ (has room_id)
└──────┬───────┘
       │ Setup Publisher + Consumer
       ▼
┌──────────────┐
│    Active    │ (media flowing)
└──────────────┘
```

### Typical Session Flow

```
1. WebSocket Connection
   └─> Client connects to ws://server:8080/ws

2. Registration
   Client → Register { username }
   Server → Registered { user_id }

3. Room Join
   Client → JoinRoom { room_id }
   Server → RoomJoined { room_id, participants }
   
4. Publishing Setup (if microphone available)
   Client → PublishOffer { sdp }
   Server → PublishAnswer { sdp }
   Client → PublishIceCandidate { candidate } (multiple)
   Server → PublishIceCandidate { candidate } (multiple)
   
5. Consuming Setup
   Client → Subscribe { room_id }
   Server → ConsumeOffer { sdp, track_mappings }
   Client → ConsumeAnswer { sdp }
   Client → ConsumeIceCandidate { candidate } (multiple)
   Server → ConsumeIceCandidate { candidate } (multiple)
   
6. Dynamic Track Events
   Server → TrackAdded { track_id, user_id, kind }
   Server → TrackRemoved { track_id }
   
7. Cleanup
   Client → LeaveRoom
   Server → RoomLeft
```

---

## Message Types

### Client → Server Messages

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    // ===== Room Management (unchanged) =====
    Register {
        username: String,
    },
    
    CreateRoom,
    
    JoinRoom {
        room_id: String,
    },
    
    LeaveRoom,
    
    Ping,
    
    // ===== Publishing (new) =====
    PublishOffer {
        sdp: String,
    },
    
    PublishAnswer {
        sdp: String,
    },
    
    PublishIceCandidate {
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    },
    
    // ===== Consuming (new) =====
    Subscribe {
        room_id: String,
    },
    
    Unsubscribe,
    
    ConsumeAnswer {
        sdp: String,
    },
    
    ConsumeIceCandidate {
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    },
    
    // ===== Quality Control (Phase 2) =====
    UpdateBandwidth {
        available_kbps: u32,
    },
    
    RequestKeyFrame {
        track_id: String,
    },
}
```

### Server → Client Messages

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    // ===== Room Management (unchanged) =====
    Registered {
        user_id: String,
    },
    
    RoomCreated {
        room_id: String,
    },
    
    RoomJoined {
        room_id: String,
        participants: Vec<ParticipantInfo>,
    },
    
    UserJoined {
        username: String,
        user_id: String,
    },
    
    UserLeft {
        username: String,
        user_id: String,
    },
    
    RoomLeft,
    
    Pong,
    
    // ===== Publishing (new) =====
    PublishAnswer {
        sdp: String,
    },
    
    PublishIceCandidate {
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    },
    
    // ===== Consuming (new) =====
    ConsumeOffer {
        sdp: String,
        track_mappings: Vec<TrackMapping>,
    },
    
    ConsumeIceCandidate {
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    },
    
    // ===== Track Lifecycle (new) =====
    TrackAdded {
        track_id: String,
        user_id: String,
        username: String,
        kind: TrackKind, // "audio" or "video"
    },
    
    TrackRemoved {
        track_id: String,
        user_id: String,
    },
    
    // ===== Quality Control (Phase 2) =====
    QualityUpdate {
        target_bitrate_kbps: u32,
        reason: String,
    },
    
    // ===== Statistics (Phase 2) =====
    StatsReport {
        timestamp: u64,
        connections: Vec<ConnectionStats>,
    },
    
    // ===== Errors =====
    Error {
        message: String,
        code: Option<String>,
    },
}
```

### Supporting Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInfo {
    pub username: String,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackMapping {
    pub track_id: String,
    pub user_id: String,
    pub username: String,
    pub kind: TrackKind,
    pub mid: String, // Media ID from SDP
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackKind {
    Audio,
    Video,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub user_id: String,
    pub bitrate_kbps: f64,
    pub packet_loss_percent: f64,
    pub rtt_ms: f64,
    pub jitter_ms: f64,
}
```

---

## Publisher Flow

### Publishing Audio to SFU

**Sequence Diagram:**

```
Client (Publisher)                     Server (SFU)
       │                                    │
       ├─── PublishOffer ──────────────────►│
       │    { sdp: "v=0..." }               │
       │                                    ├─ Create Publisher
       │                                    ├─ Create PeerConnection
       │                                    ├─ Set RemoteDescription
       │                                    ├─ Create Answer
       │◄─── PublishAnswer ─────────────────┤
       │    { sdp: "v=0..." }               │
       │                                    │
       ├─ Set RemoteDescription             │
       │                                    │
       ├─── PublishIceCandidate ───────────►│
       │    { candidate: "..." }            ├─ Add ICE candidate
       │                                    │
       │◄─── PublishIceCandidate ───────────┤
       │    { candidate: "..." }            │
       ├─ Add ICE candidate                 │
       │                                    │
       │                                    │
       ├═══ RTP Audio Packets ═════════════►│
       │    (Media flowing)                 ├─ Forward to consumers
       │                                    │
```

### State Transitions

```
Publisher States:
┌────────────┐
│    New     │
└─────┬──────┘
      │ Receive PublishOffer
      ▼
┌────────────┐
│ Connecting │
└─────┬──────┘
      │ ICE negotiation
      ▼
┌────────────┐
│ Connected  │ (media flowing)
└─────┬──────┘
      │ Track available
      ▼
┌────────────┐
│ Publishing │ (actively forwarding)
└─────┬──────┘
      │ Disconnect
      ▼
┌────────────┐
│   Closed   │
└────────────┘
```

### Client-Side Implementation

```rust
// Step 1: Create PeerConnection
let pc = create_rtc_peer_connection()?;

// Step 2: Add local audio track
let local_stream = get_user_media().await?;
for track in local_stream.get_tracks() {
    pc.add_track(&track, &local_stream)?;
}

// Step 3: Create and send offer
let offer = pc.create_offer().await?;
pc.set_local_description(&offer).await?;

send_message(ClientMessage::PublishOffer {
    sdp: offer.sdp.clone(),
})?;

// Step 4: Handle answer from server
on_message(ServerMessage::PublishAnswer { sdp }) => {
    let answer = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    answer.set_sdp(&sdp);
    pc.set_remote_description(&answer).await?;
}

// Step 5: Exchange ICE candidates
pc.set_onicecandidate(|event: RtcPeerConnectionIceEvent| {
    if let Some(candidate) = event.candidate() {
        send_message(ClientMessage::PublishIceCandidate {
            candidate: candidate.candidate(),
            sdp_mid: candidate.sdp_mid(),
            sdp_mline_index: candidate.sdp_mline_index(),
        })?;
    }
});

on_message(ServerMessage::PublishIceCandidate { candidate, .. }) => {
    let ice_candidate = RtcIceCandidateInit::new(&candidate);
    pc.add_ice_candidate(&ice_candidate).await?;
}
```

### Server-Side Implementation

```rust
async fn handle_publish_offer(
    user_id: String,
    room_id: String,
    sdp: String,
    router: Arc<SfuRouter>,
) -> Result<String> {
    // Create publisher
    let publisher = router.create_publisher(user_id.clone(), room_id.clone()).await?;
    
    // Handle offer
    let answer_sdp = publisher.handle_offer(sdp).await?;
    
    // Setup track forwarding
    let pc = publisher.peer_connection();
    pc.on_track(Box::new(move |track, _receiver, _transceiver| {
        let track_id = generate_track_id();
        
        // Forward this track to all consumers in the room
        router.forward_track(
            user_id.clone(),
            room_id.clone(),
            track_id.clone(),
            track,
        ).await?;
        
        // Notify all clients about new track
        broadcast_to_room(ServerMessage::TrackAdded {
            track_id,
            user_id: user_id.clone(),
            username: get_username(&user_id)?,
            kind: TrackKind::Audio,
        }).await?;
        
        Ok(())
    }));
    
    Ok(answer_sdp)
}
```

---

## Consumer Flow

### Consuming Audio from SFU

**Sequence Diagram:**

```
Client (Consumer)                      Server (SFU)
       │                                    │
       ├─── Subscribe ─────────────────────►│
       │    { room_id: "..." }              │
       │                                    ├─ Create Consumer
       │                                    ├─ Create PeerConnection
       │                                    ├─ Add all room tracks
       │                                    ├─ Create Offer
       │◄─── ConsumeOffer ──────────────────┤
       │    { sdp: "...",                   │
       │      track_mappings: [...] }       │
       │                                    │
       ├─ Set RemoteDescription             │
       ├─ Create Answer                     │
       ├─── ConsumeAnswer ─────────────────►│
       │    { sdp: "..." }                  │
       │                                    ├─ Set RemoteDescription
       │                                    │
       ├─── ConsumeIceCandidate ───────────►│
       │    { candidate: "..." }            ├─ Add ICE candidate
       │                                    │
       │◄─── ConsumeIceCandidate ───────────┤
       │    { candidate: "..." }            │
       ├─ Add ICE candidate                 │
       │                                    │
       │                                    │
       │◄═══ RTP Audio Packets ═════════════┤
       │    (Media flowing)                 │
       │                                    │
       │                                    │
       │◄─── TrackAdded ────────────────────┤
       │    { track_id, user_id, kind }     │
       │                                    │
```

### State Transitions

```
Consumer States:
┌────────────┐
│    New     │
└─────┬──────┘
      │ Send Subscribe
      ▼
┌────────────┐
│ Connecting │
└─────┬──────┘
      │ ICE negotiation
      ▼
┌────────────┐
│ Connected  │ (ready to receive)
└─────┬──────┘
      │ Tracks added
      ▼
┌────────────┐
│ Consuming  │ (receiving media)
└─────┬──────┘
      │ Disconnect
      ▼
┌────────────┐
│   Closed   │
└────────────┘
```

### Client-Side Implementation

```rust
// Step 1: Create PeerConnection for consuming
let pc = create_rtc_peer_connection()?;

// Step 2: Setup ontrack handler (before subscribing)
pc.set_ontrack(move |event: RtcTrackEvent| {
    let track = event.track();
    let streams = event.streams();
    
    // Extract metadata from stream ID
    // Server embeds: "audio-{user_id}"
    if let Some(stream) = streams.get(0).dyn_into::<MediaStream>().ok() {
        let stream_id = stream.id();
        let user_id = extract_user_id(&stream_id)?;
        
        // Play remote audio
        play_remote_audio(track, user_id);
    }
});

// Step 3: Subscribe to room
send_message(ClientMessage::Subscribe {
    room_id: room_id.clone(),
})?;

// Step 4: Handle offer from server
on_message(ServerMessage::ConsumeOffer { sdp, track_mappings }) => {
    // Store track mappings for later use
    for mapping in track_mappings {
        track_map.insert(mapping.mid, mapping);
    }
    
    // Set remote description
    let offer = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    offer.set_sdp(&sdp);
    pc.set_remote_description(&offer).await?;
    
    // Create answer
    let answer = pc.create_answer().await?;
    pc.set_local_description(&answer).await?;
    
    send_message(ClientMessage::ConsumeAnswer {
        sdp: answer.sdp.clone(),
    })?;
}

// Step 5: Exchange ICE candidates
pc.set_onicecandidate(|event: RtcPeerConnectionIceEvent| {
    if let Some(candidate) = event.candidate() {
        send_message(ClientMessage::ConsumeIceCandidate {
            candidate: candidate.candidate(),
            sdp_mid: candidate.sdp_mid(),
            sdp_mline_index: candidate.sdp_mline_index(),
        })?;
    }
});

on_message(ServerMessage::ConsumeIceCandidate { candidate, .. }) => {
    let ice_candidate = RtcIceCandidateInit::new(&candidate);
    pc.add_ice_candidate(&ice_candidate).await?;
}

// Step 6: Handle dynamic track events
on_message(ServerMessage::TrackAdded { track_id, user_id, username, kind }) => {
    // Track will arrive via ontrack handler
    // Update UI to show participant is publishing
    info!("User {} started publishing {}", username, kind);
}

on_message(ServerMessage::TrackRemoved { track_id, user_id }) => {
    // Remove audio element for this user
    stop_remote_audio(user_id);
}
```

### Server-Side Implementation

```rust
async fn handle_subscribe(
    user_id: String,
    room_id: String,
    router: Arc<SfuRouter>,
) -> Result<(String, Vec<TrackMapping>)> {
    // Create consumer for this user
    let consumer = router.create_consumer(user_id.clone(), room_id.clone()).await?;
    
    // Get all active publishers in the room
    let publishers = router.get_room_publishers(&room_id).await?;
    
    let mut track_mappings = Vec::new();
    
    // Add all existing tracks to consumer
    for (publisher_id, publisher) in publishers {
        let tracks = publisher.get_tracks().await;
        
        for track in tracks {
            let track_id = generate_track_id();
            
            // Create TrackLocal for forwarding
            let local_track = create_track_local_from_remote(&track)?;
            
            // Add track to consumer's PeerConnection
            consumer.add_track(local_track.clone()).await?;
            
            // Record mapping
            track_mappings.push(TrackMapping {
                track_id: track_id.clone(),
                user_id: publisher_id.clone(),
                username: get_username(&publisher_id)?,
                kind: TrackKind::Audio,
                mid: local_track.mid()?,
            });
            
            // Setup forwarding from publisher to this consumer
            start_forwarding(track, local_track).await?;
        }
    }
    
    // Create offer with all tracks
    let offer_sdp = consumer.create_offer().await?;
    
    Ok((offer_sdp, track_mappings))
}
```

---

## Transport Management

### ICE Candidate Exchange

**Purpose:** Establish NAT traversal for direct media transport

**Flow:**
```
Both Publisher and Consumer transports exchange ICE candidates:

1. Local ICE gathering triggered by setLocalDescription
2. Candidates sent as they are discovered (trickle ICE)
3. Remote peer adds candidates via addIceCandidate
4. Best candidate pair selected automatically
5. Media flows once ICE state = "connected"
```

**Message Format:**
```json
// Client → Server
{
  "type": "publish_ice_candidate",
  "candidate": "candidate:1 1 UDP 2130706431 192.168.1.100 54321 typ host",
  "sdp_mid": "0",
  "sdp_mline_index": 0
}

// Server → Client
{
  "type": "consume_ice_candidate",
  "candidate": "candidate:1 1 UDP 2130706431 10.0.0.1 12345 typ host",
  "sdp_mid": "0",
  "sdp_mline_index": 0
}
```

### Connection Management

**Reconnection Strategy:**

```
Client-side:
1. Detect connection loss (ICE state = "disconnected" or "failed")
2. Wait 2 seconds for automatic recovery
3. If not recovered, attempt reconnection:
   - Close existing PeerConnection
   - Create new PeerConnection
   - Send new PublishOffer / Subscribe
4. Maximum 3 reconnection attempts
5. If all fail, notify user and reload page

Server-side:
1. Detect disconnection via PeerConnection state
2. Keep publisher/consumer objects alive for 30 seconds
3. Allow reconnection with same user_id
4. After timeout, cleanup resources and notify room
```

**Graceful Shutdown:**

```
Client-initiated:
1. Client sends LeaveRoom message
2. Server closes all transports for this user
3. Server notifies other participants (UserLeft)
4. Client closes WebSocket connection

Server-initiated:
1. Server sends RoomLeft message (e.g., kicked)
2. Client closes all transports
3. Client returns to lobby UI
4. WebSocket remains open for rejoining
```

---

## State Machines

### Client State Machine

```
┌─────────────────────────────────────────────────────────────┐
│                        Client States                        │
└─────────────────────────────────────────────────────────────┘

States:
- Disconnected: No WebSocket connection
- Connected: WebSocket open, not registered
- Registered: Has user_id, not in room
- InRoom: In a room, no media
- Publishing: Publisher PeerConnection active
- Consuming: Consumer PeerConnection active
- Active: Both publisher and consumer active

Transitions:
┌──────────────┐ WebSocket        ┌───────────┐
│ Disconnected │─────────────────►│ Connected │
└──────────────┘      open        └─────┬─────┘
                                        │ Register
                                        ▼
                                  ┌────────────┐
                                  │ Registered │
                                  └─────┬──────┘
                                        │ JoinRoom
                                        ▼
                                  ┌─────────┐
                                  │ InRoom  │
                                  └────┬────┘
                                       │
                  ┌────────────────────┼────────────────────┐
                  │                    │                    │
         PublishOffer            Subscribe          (Both)
                  │                    │                    │
                  ▼                    ▼                    ▼
           ┌────────────┐       ┌───────────┐       ┌────────┐
           │ Publishing │       │ Consuming │       │ Active │
           └────────────┘       └───────────┘       └────────┘
```

### Server State Machine (Per User)

```
┌─────────────────────────────────────────────────────────────┐
│                    Server-Side User States                  │
└─────────────────────────────────────────────────────────────┘

States:
- Connected: WebSocket connection established
- Registered: User has user_id
- InRoom: User is in a room
- HasPublisher: Publisher PeerConnection created
- HasConsumer: Consumer PeerConnection created
- FullyConnected: Both transports active, media flowing

Transitions:
┌───────────┐ Register      ┌────────────┐
│ Connected │──────────────►│ Registered │
└───────────┘               └──────┬─────┘
                                   │ JoinRoom
                                   ▼
                             ┌──────────┐
                             │  InRoom  │
                             └────┬─────┘
                                  │
             ┌────────────────────┼────────────────────┐
             │                    │                    │
    PublishOffer            Subscribe          (Both)
             │                    │                    │
             ▼                    ▼                    ▼
      ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
      │ HasPublisher │    │ HasConsumer  │    │ FullyConnected│
      └──────────────┘    └──────────────┘    └──────────────┘

Cleanup triggers:
- LeaveRoom message
- WebSocket disconnect
- PeerConnection failure (after retry timeout)
```

### Publisher PeerConnection State

```
┌────────┐ offer      ┌─────────────┐ ICE      ┌───────────┐
│  New   │───────────►│ Negotiating │─────────►│ Connected │
└────────┘            └─────────────┘  success  └────┬──────┘
                                                      │
                                                ontrack
                                                      │
                                                      ▼
                                                ┌──────────┐
                                                │ Tracking │
                                                └────┬─────┘
                                                     │ close
                                                     ▼
                                                ┌────────┐
                                                │ Closed │
                                                └────────┘
```

### Consumer PeerConnection State

```
┌────────┐ subscribe  ┌─────────────┐ answer   ┌───────────┐
│  New   │───────────►│ Negotiating │─────────►│ Connected │
└────────┘            └─────────────┘           └────┬──────┘
                                                      │
                                                addTrack
                                                      │
                                                      ▼
                                                ┌──────────┐
                                                │ Streaming│
                                                └────┬─────┘
                                                     │ close
                                                     ▼
                                                ┌────────┐
                                                │ Closed │
                                                └────────┘
```

---

## Error Handling

### Error Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Error {
        message: String,
        code: Option<String>,
    },
}

// Error codes
pub mod error_codes {
    pub const NOT_REGISTERED: &str = "NOT_REGISTERED";
    pub const NOT_IN_ROOM: &str = "NOT_IN_ROOM";
    pub const ROOM_NOT_FOUND: &str = "ROOM_NOT_FOUND";
    pub const INVALID_SDP: &str = "INVALID_SDP";
    pub const PEER_CONNECTION_FAILED: &str = "PEER_CONNECTION_FAILED";
    pub const TRACK_NOT_FOUND: &str = "TRACK_NOT_FOUND";
    pub const ALREADY_PUBLISHING: &str = "ALREADY_PUBLISHING";
    pub const NOT_PUBLISHING: &str = "NOT_PUBLISHING";
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
}
```

### Error Scenarios

**Client-Side Errors:**

1. **Invalid State:**
   ```json
   {
     "type": "error",
     "message": "Cannot publish: not in a room",
     "code": "NOT_IN_ROOM"
   }
   ```
   - **Cause:** Client sent PublishOffer before joining room
   - **Recovery:** Join room first, then publish

2. **SDP Negotiation Failure:**
   ```json
   {
     "type": "error",
     "message": "Failed to parse SDP offer",
     "code": "INVALID_SDP"
   }
   ```
   - **Cause:** Malformed SDP string
   - **Recovery:** Regenerate offer and retry

3. **PeerConnection Failure:**
   ```json
   {
     "type": "error",
     "message": "Publisher connection failed",
     "code": "PEER_CONNECTION_FAILED"
   }
   ```
   - **Cause:** ICE negotiation failed, network issues
   - **Recovery:** Close connection, wait 2s, retry

**Server-Side Error Handling:**

```rust
async fn handle_publish_offer(/* ... */) -> Result<String> {
    // Validate state
    if !is_in_room(&user_id).await? {
        return Err(Error::NotInRoom);
    }
    
    // Validate SDP
    if !is_valid_sdp(&sdp) {
        return Err(Error::InvalidSdp);
    }
    
    // Create publisher with timeout
    let publisher = tokio::time::timeout(
        Duration::from_secs(10),
        router.create_publisher(user_id, room_id)
    ).await??;
    
    // Handle offer with error recovery
    match publisher.handle_offer(sdp).await {
        Ok(answer) => Ok(answer),
        Err(e) => {
            // Cleanup failed publisher
            router.remove_publisher(&user_id).await?;
            Err(Error::PeerConnectionFailed(e))
        }
    }
}
```

### Client Error Recovery

```rust
// Automatic retry logic
async fn publish_with_retry(stream: MediaStream, max_retries: u32) -> Result<()> {
    let mut attempts = 0;
    
    loop {
        match setup_publisher(stream.clone()).await {
            Ok(_) => return Ok(()),
            Err(e) if attempts < max_retries => {
                warn!("Publish attempt {} failed: {:?}", attempts + 1, e);
                attempts += 1;
                
                // Exponential backoff
                let delay = Duration::from_secs(2u64.pow(attempts));
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

---

## Message Examples

### Registration & Room Join

**Register:**
```json
// Client → Server
{
  "type": "register",
  "username": "Alice"
}

// Server → Client
{
  "type": "registered",
  "user_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Join Room:**
```json
// Client → Server
{
  "type": "join_room",
  "room_id": "room-123"
}

// Server → Client
{
  "type": "room_joined",
  "room_id": "room-123",
  "participants": [
    {
      "username": "Bob",
      "user_id": "660e8400-e29b-41d4-a716-446655440001"
    },
    {
      "username": "Charlie",
      "user_id": "770e8400-e29b-41d4-a716-446655440002"
    }
  ]
}
```

### Publishing Flow

**Publish Offer:**
```json
// Client → Server
{
  "type": "publish_offer",
  "sdp": "v=0\r\no=- 123456789 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\na=group:BUNDLE 0\r\na=msid-semantic: WMS stream\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111\r\nc=IN IP4 0.0.0.0\r\na=rtcp:9 IN IP4 0.0.0.0\r\na=ice-ufrag:abcd\r\na=ice-pwd:efgh1234\r\na=fingerprint:sha-256 AB:CD:EF...\r\na=setup:actpass\r\na=mid:0\r\na=sendonly\r\na=rtcp-mux\r\na=rtpmap:111 opus/48000/2\r\na=fmtp:111 minptime=10;useinbandfec=1\r\n"
}

// Server → Client
{
  "type": "publish_answer",
  "sdp": "v=0\r\no=- 987654321 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\na=group:BUNDLE 0\r\na=msid-semantic: WMS\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111\r\nc=IN IP4 0.0.0.0\r\na=rtcp:9 IN IP4 0.0.0.0\r\na=ice-ufrag:ijkl\r\na=ice-pwd:mnop5678\r\na=fingerprint:sha-256 12:34:56...\r\na=setup:active\r\na=mid:0\r\na=recvonly\r\na=rtcp-mux\r\na=rtpmap:111 opus/48000/2\r\n"
}
```

**ICE Candidates:**
```json
// Client → Server
{
  "type": "publish_ice_candidate",
  "candidate": "candidate:1 1 UDP 2130706431 192.168.1.100 54321 typ host",
  "sdp_mid": "0",
  "sdp_mline_index": 0
}

// Server → Client
{
  "type": "publish_ice_candidate",
  "candidate": "candidate:2 1 UDP 1694498815 203.0.113.1 12345 typ srflx raddr 192.168.1.1 rport 54321",
  "sdp_mid": "0",
  "sdp_mline_index": 0
}
```

### Consuming Flow

**Subscribe:**
```json
// Client → Server
{
  "type": "subscribe",
  "room_id": "room-123"
}

// Server → Client
{
  "type": "consume_offer",
  "sdp": "v=0\r\no=- 111222333 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\na=group:BUNDLE 0 1\r\na=msid-semantic: WMS\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111\r\nc=IN IP4 0.0.0.0\r\na=rtcp:9 IN IP4 0.0.0.0\r\na=ice-ufrag:qrst\r\na=ice-pwd:uvwx9012\r\na=fingerprint:sha-256 78:90:AB...\r\na=setup:actpass\r\na=mid:0\r\na=sendonly\r\na=rtcp-mux\r\na=rtpmap:111 opus/48000/2\r\na=msid:bob-stream audio-track-1\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111\r\nc=IN IP4 0.0.0.0\r\na=rtcp:9 IN IP4 0.0.0.0\r\na=ice-ufrag:qrst\r\na=ice-pwd:uvwx9012\r\na=fingerprint:sha-256 78:90:AB...\r\na=setup:actpass\r\na=mid:1\r\na=sendonly\r\na=rtcp-mux\r\na=rtpmap:111 opus/48000/2\r\na=msid:charlie-stream audio-track-2\r\n",
  "track_mappings": [
    {
      "track_id": "track-001",
      "user_id": "660e8400-e29b-41d4-a716-446655440001",
      "username": "Bob",
      "kind": "audio",
      "mid": "0"
    },
    {
      "track_id": "track-002",
      "user_id": "770e8400-e29b-41d4-a716-446655440002",
      "username": "Charlie",
      "kind": "audio",
      "mid": "1"
    }
  ]
}
```

**Consume Answer:**
```json
// Client → Server
{
  "type": "consume_answer",
  "sdp": "v=0\r\no=- 444555666 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\na=group:BUNDLE 0 1\r\na=msid-semantic: WMS\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111\r\nc=IN IP4 0.0.0.0\r\na=rtcp:9 IN IP4 0.0.0.0\r\na=ice-ufrag:yzab\r\na=ice-pwd:cdef3456\r\na=fingerprint:sha-256 CD:EF:01...\r\na=setup:active\r\na=mid:0\r\na=recvonly\r\na=rtcp-mux\r\na=rtpmap:111 opus/48000/2\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111\r\nc=IN IP4 0.0.0.0\r\na=rtcp:9 IN IP4 0.0.0.0\r\na=ice-ufrag:yzab\r\na=ice-pwd:cdef3456\r\na=fingerprint:sha-256 CD:EF:01...\r\na=setup:active\r\na=mid:1\r\na=recvonly\r\na=rtcp-mux\r\na=rtpmap:111 opus/48000/2\r\n"
}
```

### Dynamic Track Events

**Track Added:**
```json
// Server → Client (broadcast to all consumers in room)
{
  "type": "track_added",
  "track_id": "track-003",
  "user_id": "880e8400-e29b-41d4-a716-446655440003",
  "username": "Diana",
  "kind": "audio"
}
```

**Track Removed:**
```json
// Server → Client (broadcast to all consumers in room)
{
  "type": "track_removed",
  "track_id": "track-001",
  "user_id": "660e8400-e29b-41d4-a716-446655440001"
}
```

### Quality Control (Phase 2)

**Bandwidth Update:**
```json
// Client → Server
{
  "type": "update_bandwidth",
  "available_kbps": 128
}

// Server → Client
{
  "type": "quality_update",
  "target_bitrate_kbps": 64,
  "reason": "Limited bandwidth detected"
}
```

**Statistics Report:**
```json
// Server → Client (periodic, every 5 seconds)
{
  "type": "stats_report",
  "timestamp": 1704720000000,
  "connections": [
    {
      "user_id": "660e8400-e29b-41d4-a716-446655440001",
      "bitrate_kbps": 48.5,
      "packet_loss_percent": 0.8,
      "rtt_ms": 35.2,
      "jitter_ms": 4.1
    },
    {
      "user_id": "770e8400-e29b-41d4-a716-446655440002",
      "bitrate_kbps": 52.3,
      "packet_loss_percent": 1.2,
      "rtt_ms": 42.7,
      "jitter_ms": 3.8
    }
  ]
}
```

---

## Protocol Evolution

### Version Negotiation (Future)

**For backward compatibility in later versions:**

```json
// Client → Server (first message)
{
  "type": "hello",
  "protocol_version": "1.0",
  "capabilities": ["audio", "bandwidth-adaptation"]
}

// Server → Client
{
  "type": "welcome",
  "protocol_version": "1.0",
  "server_capabilities": ["audio", "bandwidth-adaptation", "recording"]
}
```

### Extension Points

**Custom metadata in track mappings:**
```json
{
  "track_id": "track-001",
  "user_id": "...",
  "username": "Bob",
  "kind": "audio",
  "mid": "0",
  "metadata": {
    "muted": false,
    "speaking": true,
    "audio_level": 75.5
  }
}
```

**Application-specific messages:**
```json
{
  "type": "custom",
  "app_type": "chat",
  "data": {
    "message": "Hello, world!"
  }
}
```

---

## Summary

**Key Protocol Changes from P2P:**
- ✅ Separate publishing and consuming connections
- ✅ Server-initiated consumer offers
- ✅ Explicit track lifecycle management
- ✅ Stateless message design
- ✅ Room-centric architecture
- ✅ Bandwidth adaptation support (Phase 2)

**Implementation Priority:**
1. Phase 1: Publishing + Consuming core messages
2. Phase 2: Quality control messages
3. Phase 3: Simulcast negotiation (future)
4. Phase 4: Monitoring and stats messages

---

**Document Version History:**
- v1.0 (2026-01-08): Initial protocol specification

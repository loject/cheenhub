# Voice Messenger PoC - SFU Architecture (Phase 1 MVP)

A proof of concept for a voice messenger built with Rust. This implementation features a **Selective Forwarding Unit (SFU)** architecture using **webrtc-rs** on the backend for scalable multi-party audio communication.

## Tech Stack

- **Backend**: Rust + Axum + WebSocket + webrtc-rs (SFU)
- **Frontend**: Dioxus (WASM) + Web Audio API + WebRTC

## Architecture Overview

### SFU (Selective Forwarding Unit) Topology

This project uses an SFU architecture instead of traditional P2P mesh topology for better scalability:

```
User A → Publisher → SFU Server → Consumer → User B
                                 → Consumer → User C
                                 → Consumer → User D
```

**Benefits over P2P Mesh:**
- **Scalability**: Each user maintains 1 upload connection (publisher) + N download connections (consumers)
- **Server-side control**: Bandwidth adaptation and quality control at the SFU
- **Lower client bandwidth**: No need to upload to every peer
- **Better for mobile**: Reduced CPU and network usage on client devices

**Key Concepts:**
- **Publisher**: Client-to-server connection that sends audio
- **Consumer**: Server-to-client connection that receives audio from a specific publisher
- **Track forwarding**: RTP packets are forwarded directly from publisher to consumers (no transcoding)

### Backend (webrtc-rs)

The backend implements a full-featured SFU using webrtc-rs:

- **SfuRouter**: Central routing logic managing publishers and consumers
- **Publisher**: Receives audio tracks from clients via WebRTC
- **Consumer**: Forwards audio tracks to clients via WebRTC
- **Direct RTP forwarding**: Minimal latency by forwarding packets without transcoding
- **ICE/STUN support**: NAT traversal using public STUN servers

### Frontend (Browser WebRTC)

The frontend uses native browser WebRTC APIs:

- **Publisher connection**: Sends local microphone audio to SFU
- **Consumer connections**: One per remote participant, receives their audio
- **Audio playback**: Automatic playback of received tracks
- **Statistics**: Real-time connection metrics (bitrate, jitter, packet loss, RTT)

## Project Structure

```
.
├── backend/                    # SFU server implementation
│   ├── src/
│   │   ├── main.rs            # WebSocket signaling + room management
│   │   └── sfu/               # SFU implementation
│   │       ├── mod.rs         # Module exports
│   │       ├── types.rs       # Type definitions
│   │       ├── router.rs      # SfuRouter - central routing logic
│   │       ├── publisher.rs   # Publisher - client→server audio
│   │       └── consumer.rs    # Consumer - server→client audio
│   └── Cargo.toml
├── frontend/                   # Dioxus WASM application
│   ├── src/
│   │   └── main.rs            # Frontend with SFU publisher/consumer logic
│   ├── style.css              # UI styles
│   ├── Dioxus.toml            # Dioxus configuration
│   └── Cargo.toml
├── plans/                      # Documentation and roadmaps
│   ├── sfu-migration-plan.md
│   ├── sfu-signaling-protocol.md
│   └── sfu-implementation-roadmap.md
├── PHASE1_STATUS.md           # Current implementation status
└── Cargo.toml                 # Workspace configuration
```

## Prerequisites

- Rust (latest stable version)
- Dioxus CLI: `cargo install dioxus-cli`
- A browser with WebRTC support (Chrome, Firefox, Edge, Safari)

## Running the Application

### 1. Start the Backend Server

Open a terminal and run:

```bash
cd backend
cargo run
```

The server will start on `http://localhost:8080` with:
- WebSocket signaling endpoint at `/ws`
- SFU router for WebRTC connections

You should see:
```
WebSocket server listening on 0.0.0.0:8080
```

### 2. Start the Frontend Application

Open a **new terminal** and run:

```bash
cd frontend
dx serve
```

The Dioxus development server will start (typically on `http://localhost:8080` or another available port).

Open your browser and navigate to the URL shown in the terminal output.

### 3. Testing with Multiple Users

To test multi-party communication:
1. Open the app in multiple browser windows/tabs or different browsers
2. Connect each with a different username
3. Create a room in one window and copy the room link
4. Join the room from other windows using the link
5. Grant microphone access in each window
6. All users should hear each other through the SFU

## Using the Application

### 1. Connect to Server
1. Enter your username in the text field
2. Click "Connect to Server" button
3. The status will change from "Disconnected" to "Connected"
4. You will be automatically registered with a unique user ID

### 2. Enable Microphone
1. Click "Request Microphone Access" button
2. Your browser will ask for permission to access the microphone
3. Grant permission
4. The microphone status will change to "Allowed ✓"
5. You will see a real-time audio level indicator

### 3. Create or Join a Room

**Option A: Create a New Room**
1. Click "Create New Room" button
2. A new room will be created with a unique ID
3. Your microphone will automatically start publishing to the SFU
4. Click "📋 Copy Room Link" to copy the shareable link
5. Share this link with others to invite them

**Option B: Join an Existing Room**
1. Enter the room ID in the input field (or it will be auto-filled from URL)
2. Click "Join Room" button
3. If you have microphone access, you'll start publishing automatically
4. You'll see the list of participants and hear their audio

### 4. In the Room

- **Audio Transmission**: Your audio is sent to the SFU, which forwards it to all other participants
- **Audio Reception**: You receive audio from all other participants through individual consumer connections
- **Participant List**: See all users in the room with real-time audio level indicators
- **Statistics**: Toggle "Show detailed statistics" to see connection metrics
- **Mute/Unmute**: Control your microphone with the mute button
- **Leave Room**: Click "Leave Room" to disconnect

## Features

### Phase 1 MVP (Current - SFU Implementation)

✅ **Backend SFU (webrtc-rs)**
- Full SFU implementation with Publisher/Consumer model
- Direct RTP packet forwarding (minimal latency)
- ICE candidate handling for NAT traversal
- Track-based audio routing
- Multiple consumers per publisher

✅ **Frontend SFU Integration**
- Publisher connection (sends audio to SFU)
- Consumer connections (receives audio from SFU)
- Automatic connection management
- SFU-based signaling protocol

✅ **Room Management**
- Create/join rooms with unique IDs
- Shareable room links
- Real-time participant list
- User join/leave notifications

✅ **Audio Features**
- Microphone access with permission handling
- Real-time audio level visualization (local + remote)
- Mute/unmute functionality
- Low-latency audio capture and playback
- Opus codec with optimized settings

✅ **Statistics & Monitoring**
- Real-time connection statistics
- Audio bitrate monitoring
- Packet loss, jitter, RTT metrics
- Connection state tracking
- Codec information display

### Previous Iterations (Foundation)

✅ **Iteration 1**: WebSocket connectivity and basic messaging
✅ **Iteration 2**: Microphone access and audio level visualization  
✅ **Iteration 3**: Room management and participant tracking
✅ **Iteration 4**: P2P WebRTC audio transmission (replaced by SFU)

## Development Notes

### Backend Architecture

**SFU Components:**
- **SfuRouter** (`sfu/router.rs`): Central coordinator managing all publishers and consumers
- **Publisher** (`sfu/publisher.rs`): Handles incoming WebRTC connections and audio tracks from clients
- **Consumer** (`sfu/consumer.rs`): Handles outgoing WebRTC connections and forwards audio to clients
- **Message Handlers** (`main.rs`): WebSocket signaling for SFU operations

**Message Flow:**
1. Client joins room → receives list of existing publishers
2. Client creates publisher → server sends SDP offer → client sends answer
3. Server notifies other clients of new publisher
4. Other clients create consumers → receive SDP offers → send answers
5. Audio flows: Client → Publisher → SFU → Consumers → Other Clients

**Key Dependencies:**
- `webrtc = "0.11"`: Full WebRTC implementation in Rust
- `tokio`: Async runtime
- `axum`: Web framework and WebSocket support

### Frontend Architecture

**State Management:**
- `publisher_connection`: Single connection sending audio to SFU
- `consumer_connections`: HashMap of connections receiving audio (one per remote user)
- `participant_audio_levels`: Real-time audio levels for visualization
- `connection_stats`: Network statistics per connection

**Message Handlers:**
- `PublisherCreated`: Creates publisher connection, adds local track, sends answer
- `AudioPublished`: Confirms successful audio publication
- `NewPublisher`: Notifies of new remote publisher, requests consumer
- `ConsumerCreated`: Creates consumer connection, receives remote track
- `ICE Candidates`: Handles NAT traversal for both publisher and consumers

**Audio Processing:**
- Low-latency AudioContext (10ms latency hint)
- 48kHz sample rate
- Opus codec with FEC and low-latency settings
- Real-time FFT analysis for audio level meters

### Signaling Protocol

All signaling happens over WebSocket using JSON messages:

**Client → Server:**
- `CreatePublisher`: Request to create publisher connection
- `PublishAudio`: Send SDP answer for publisher
- `CreateConsumer`: Request consumer for specific publisher
- `ConsumerAnswer`: Send SDP answer for consumer
- `PublisherIceCandidate`: ICE candidate for publisher
- `ConsumerIceCandidate`: ICE candidate for consumer

**Server → Client:**
- `PublisherCreated`: SDP offer for publisher connection
- `AudioPublished`: Confirmation with track ID
- `NewPublisher`: Notification of new remote publisher
- `ConsumerCreated`: SDP offer for consumer connection
- `PublisherIceCandidate`: ICE candidate from server
- `ConsumerIceCandidate`: ICE candidate from server

## Future Enhancements (Phase 2+)

### Phase 2: Advanced SFU Features
- [ ] Dynamic bandwidth adaptation based on network conditions
- [ ] Simulcast support (multiple quality levels)
- [ ] Advanced statistics and monitoring dashboard
- [ ] Automatic quality adjustment
- [ ] Network congestion detection

### Phase 3: Production Readiness
- [ ] Redis for distributed state management
- [ ] CockroachDB for persistent storage
- [ ] TURN server for restrictive NATs
- [ ] Load balancing across multiple SFU instances
- [ ] Horizontal scaling architecture

### Phase 4: Advanced Features
- [ ] Screen sharing
- [ ] Video support
- [ ] Recording capabilities
- [ ] E2E encryption
- [ ] Mobile app support

## Troubleshooting

### Backend Issues

**Port already in use:**
```bash
# Kill process on port 8080
sudo lsof -ti:8080 | xargs kill -9
```

**WebRTC connection fails:**
- Check STUN server connectivity
- Verify ICE candidates are being exchanged
- Check firewall settings
- Look for errors in backend logs

**High latency or audio quality issues:**
- Check network conditions (RTT, packet loss)
- Verify Opus codec is being used
- Check server CPU usage
- Monitor connection statistics

### Frontend Issues

**Cannot connect to server:**
- Ensure backend is running first
- Check browser console for WebSocket errors
- Verify WebSocket URL is correct

**No audio from remote participants:**
- Check that publisher is successfully created
- Verify consumer connections are established
- Check browser console for WebRTC errors
- Ensure autoplay is allowed in browser

**Microphone not working:**
- Grant microphone permissions in browser
- Check that no other app is using the microphone
- Try refreshing the page
- Check browser compatibility

**ICE connection failures:**
- STUN servers might be unreachable
- Network might block WebRTC (VPN/corporate firewall)
- Try from a different network
- Check ICE connection state in statistics view

### Debugging Tips

1. **Enable verbose logging**: Check browser console and backend logs
2. **Monitor statistics**: Toggle "Show detailed statistics" in the UI
3. **Check ICE state**: Look for "connected" or "completed" state
4. **Verify codec**: Ensure Opus is being used (shown in stats)
5. **Network inspection**: Use browser DevTools Network tab to monitor WebSocket

## Browser Compatibility

This application requires a modern browser with support for:
- WebAssembly (WASM)
- WebSocket
- Web Audio API
- getUserMedia API
- WebRTC (RTCPeerConnection)
- Clipboard API

Tested on:
- Chrome/Chromium 90+
- Firefox 88+
- Edge 90+
- Safari 14+ (may have limitations)

## Performance Characteristics

**Typical Metrics (LAN environment):**
- End-to-end latency: 30-50ms
- Audio bitrate: 32-64 kbps (Opus)
- Packet loss tolerance: < 5%
- Jitter: < 10ms

**Scalability:**
- Per-user bandwidth: 1 upload + N downloads (N = number of participants)
- Server bandwidth: Linear with number of connections
- CPU usage: Minimal (no transcoding, just forwarding)

## License

This is a proof of concept project.

## Documentation

See also:
- [`PHASE1_STATUS.md`](PHASE1_STATUS.md) - Current implementation status
- [`plans/sfu-migration-plan.md`](plans/sfu-migration-plan.md) - Migration plan from P2P to SFU
- [`plans/sfu-signaling-protocol.md`](plans/sfu-signaling-protocol.md) - Detailed signaling protocol
- [`plans/sfu-implementation-roadmap.md`](plans/sfu-implementation-roadmap.md) - Full roadmap

# Voice Messenger PoC (Iteration 3)

A proof of concept for a voice messenger built with Rust. This iteration features WebSocket connectivity, microphone access with audio level visualization, and room management with shareable links.

## Tech Stack

- **Backend**: Rust + Axum + WebSocket
- **Frontend**: Dioxus (WASM) + Web Audio API

## Project Structure

```
.
├── backend/          # WebSocket server with room management
│   ├── src/
│   │   └── main.rs   # Server implementation
│   └── Cargo.toml
├── frontend/         # Dioxus WASM application
│   ├── src/
│   │   └── main.rs   # Frontend implementation
│   ├── style.css     # UI styles
│   ├── Dioxus.toml   # Dioxus configuration
│   └── Cargo.toml
└── Cargo.toml        # Workspace configuration
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

The WebSocket server will start on `http://localhost:8080` with WebSocket endpoint at `/ws`.

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

The Dioxus development server will start (typically on a different port like `http://localhost:8080`).

Open your browser and navigate to the URL shown in the terminal output.

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
3. Click "📋 Copy Room Link" to copy the shareable link
4. Share this link with others to invite them

**Option B: Join an Existing Room**
1. Enter the room ID in the input field (or the room ID will be auto-filled if you opened a room link)
2. Click "Join Room" button
3. You will see the list of participants already in the room

### 4. Room Features
- View all participants in the room
- See when users join or leave in real-time
- Leave the room at any time with "Leave Room" button
- Room links automatically use the current host (works on localhost with different ports)

## Features

### Iteration 1
- ✅ WebSocket connection between frontend and backend
- ✅ Basic ping/pong message handling
- ✅ Connection status indicator
- ✅ User-friendly UI with minimal styling
- ✅ Comprehensive logging

### Iteration 2
- ✅ Microphone access request via Web Audio API
- ✅ Real-time audio level visualization
- ✅ Microphone status indicator (Not requested / Requesting / Allowed / Denied)
- ✅ Visual audio meter with gradient colors
- ✅ Automatic audio analysis using AnalyserNode

### Iteration 3 (Current)
- ✅ Room creation with unique IDs (UUID)
- ✅ Room joining via ID or shareable link
- ✅ Shareable room links with dynamic base URL
- ✅ Real-time participant list
- ✅ User join/leave notifications
- ✅ In-memory room management
- ✅ URL parameter parsing for auto-join
- ✅ Copy to clipboard functionality

## Development Notes

### Backend

- Server runs on port **8080**
- WebSocket endpoint: `/ws`
- Message types (JSON):
  - `register`: Register user with username
  - `create_room`: Create new room
  - `join_room`: Join room by ID
  - `leave_room`: Leave current room
- In-memory storage:
  - Rooms: HashMap<room_id, Room>
  - Users: HashMap<user_id, User>
  - User-Room mapping: HashMap<user_id, room_id>
- Automatic cleanup on disconnect

### Frontend

- Built with Dioxus web framework (WASM target)
- WebSocket client with dynamic URL (uses current host + port 8080)
- URL parsing for room parameter (`?room=<id>`)
- Clipboard API for copying room links
- Microphone access via `navigator.mediaDevices.getUserMedia()`
- Audio analysis using Web Audio API
- UI sections:
  - Server and microphone status
  - Audio level meter
  - Room management (create/join/leave)
  - Participants list
  - Instructions

## Future Iterations

- **Iteration 4**: WebRTC peer-to-peer audio transmission between participants
- **Iteration 5**: Multiple users per room with audio mixing (SFU/Mesh topology)
- **Iteration 6**: UI improvements and audio controls (mute/unmute, volume)
- Redis for session management
- CockroachDB for persistent storage
- Server sharding for scalability

## Troubleshooting

### Port already in use
- Make sure no other application is using port 8080 for the backend
- The frontend dev server will automatically use a different port
- Room links will automatically adapt to the frontend's port

### Frontend won't connect
- Ensure the backend server is running first
- Check browser console for error messages
- Verify WebSocket connection to port 8080

### Microphone permission denied
- Check your browser's microphone permissions (usually in address bar)
- Some browsers require HTTPS for microphone access in production
- Make sure another application isn't using your microphone

### Cannot join room
- Verify the room ID is correct
- Ensure you're connected to the server (username entered and connected)
- Check that the room still exists (rooms are in-memory only)

### Room link doesn't work
- Make sure both users are connected to the same backend server
- Room links use the current host - if accessing from different machines, adjust accordingly
- Rooms are temporary (in-memory) - they disappear when the server restarts

### Dioxus CLI not found
```bash
cargo install dioxus-cli
```

## Browser Compatibility

This application requires a modern browser with support for:
- WebAssembly (WASM)
- WebSocket
- Web Audio API
- getUserMedia API
- Clipboard API
- URLSearchParams

Tested on:
- Chrome/Chromium 90+
- Firefox 88+
- Edge 90+
- Safari 14+

## Architecture Notes

### Scalability Considerations (Future)

The current implementation uses in-memory storage and is designed as a proof of concept. For production:

1. **Sharding Strategy**: Rooms are isolated units, making them ideal for sharding
2. **State Management**: Will migrate to Redis for distributed state
3. **Persistence**: CockroachDB for user data and room history
4. **WebRTC Topology**: Will implement SFU (Selective Forwarding Unit) for multi-party calls

### Dynamic URL Handling

The frontend automatically adapts to the current host and port:
- WebSocket URL: Constructed from `window.location` (uses backend port 8080)
- Share URLs: Use current frontend host and port
- This allows the app to work on different environments (localhost, staging, production)

## License

This is a proof of concept project.

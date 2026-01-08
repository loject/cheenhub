# Voice Messenger PoC (Iteration 2)

A proof of concept for a voice messenger built with Rust. This iteration features WebSocket connectivity and microphone access with audio level visualization.

## Tech Stack

- **Backend**: Rust + Axum + WebSocket
- **Frontend**: Dioxus (WASM) + Web Audio API

## Project Structure

```
.
├── backend/          # WebSocket server
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

The Dioxus development server will start (typically on `http://localhost:8080` or another available port).

Open your browser and navigate to the URL shown in the terminal output (e.g., `http://localhost:8080`).

## Using the Application

### Basic Connection
1. Enter a username in the text field
2. Click "Connect to Server" button
3. The status will change from "Disconnected" to "Connected"
4. The application will automatically send a `ping` message
5. The server will respond with `pong`

### Microphone Access
1. Click "Request Microphone Access" button
2. Your browser will ask for permission to access the microphone
3. Grant permission
4. The microphone status will change to "Allowed ✓"
5. You will see a real-time audio level indicator showing your microphone input
6. Speak into your microphone to see the audio level bar animate

### Audio Level Indicator
- The green-to-red gradient bar shows your current audio input level
- The bar updates 20 times per second for smooth visualization
- Green indicates low levels, yellow medium, red high levels

## Features

### Iteration 1
- ✅ WebSocket connection between frontend and backend
- ✅ Basic ping/pong message handling
- ✅ Connection status indicator
- ✅ User-friendly UI with minimal styling
- ✅ Comprehensive logging

### Iteration 2 (Current)
- ✅ Microphone access request via Web Audio API
- ✅ Real-time audio level visualization
- ✅ Microphone status indicator (Not requested / Requesting / Allowed / Denied)
- ✅ Visual audio meter with gradient colors
- ✅ Automatic audio analysis using AnalyserNode

## Development Notes

### Backend

- Server runs on port **8080**
- WebSocket endpoint: `/ws`
- Handles text messages: `ping` → responds with `pong`, echoes all other messages
- Logs all connections, disconnections, and messages

### Frontend

- Built with Dioxus web framework (WASM target)
- WebSocket client connects to `ws://localhost:8080/ws`
- Uses browser's native WebSocket API via `web-sys`
- Microphone access via `navigator.mediaDevices.getUserMedia()`
- Audio analysis using Web Audio API's `AudioContext` and `AnalyserNode`
- UI includes:
  - Username input
  - Server connection button and status
  - Microphone request button and status
  - Real-time audio level meter

## Future Iterations

- **Iteration 3**: WebRTC peer-to-peer connection for audio transmission
- **Iteration 4**: Room management (in-memory)
- **Iteration 5**: Multiple users per room
- **Iteration 6**: UI improvements and audio controls (mute/unmute)
- Redis for session management
- CockroachDB for persistent storage
- Server sharding for scalability

## Troubleshooting

### Port already in use
- Make sure no other application is using port 8080
- Or modify the port in `backend/src/main.rs` (line with `SocketAddr::from`)

### Frontend won't connect
- Ensure the backend server is running first
- Check browser console for error messages
- Verify WebSocket URL matches the backend address

### Microphone permission denied
- Check your browser's microphone permissions (usually in address bar)
- Some browsers require HTTPS for microphone access in production
- Make sure another application isn't using your microphone

### Audio level not showing
- Ensure microphone permission was granted
- Try speaking louder or checking microphone input level in system settings
- Check browser console (F12) for any errors

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

Tested on:
- Chrome/Chromium 90+
- Firefox 88+
- Edge 90+
- Safari 14+

## License

This is a proof of concept project.

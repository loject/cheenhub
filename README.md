# Voice Messenger PoC (Iteration 1)

A proof of concept for a voice messenger built with Rust. This is the first iteration featuring basic WebSocket connectivity.

## Tech Stack

- **Backend**: Rust + Axum + WebSocket
- **Frontend**: Dioxus (WASM)

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

1. Enter a username in the text field
2. Click "Connect to Server" button
3. The status will change from "Disconnected" to "Connected"
4. The application will automatically send a `ping` message
5. The server will respond with `pong`
6. Check the browser console (F12) for detailed logs

## Features (Iteration 1)

- ✅ WebSocket connection between frontend and backend
- ✅ Basic ping/pong message handling
- ✅ Connection status indicator
- ✅ User-friendly UI with minimal styling
- ✅ Comprehensive logging

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
- UI includes username input, connect button, and status indicator

## Future Iterations

- WebRTC for real-time audio
- Multiple user support and rooms
- Redis for session management
- CockroachDB for persistent storage
- Server sharding for scalability

## Troubleshooting

**Port already in use:**
- Make sure no other application is using port 8080
- Or modify the port in `backend/src/main.rs` (line with `SocketAddr::from`)

**Frontend won't connect:**
- Ensure the backend server is running first
- Check browser console for error messages
- Verify WebSocket URL matches the backend address

**Dioxus CLI not found:**
```bash
cargo install dioxus-cli
```

## License

This is a proof of concept project.

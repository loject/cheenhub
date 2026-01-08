use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tracing::{info, warn};
use tracing_subscriber;

#[tokio::main]
async fn main() {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Build application router with WebSocket endpoint
    let app = Router::new()
        .route("/ws", get(ws_handler));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("WebSocket server listening on {}", addr);

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

/// WebSocket upgrade handler
async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

/// Handle individual WebSocket connection
async fn handle_socket(mut socket: WebSocket) {
    info!("Client connected");

    // Process incoming messages
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                info!("Received text message: {}", text);
                
                // Handle ping message specifically
                if text.trim() == "ping" {
                    info!("Responding with pong");
                    if socket.send(Message::Text("pong".to_string())).await.is_err() {
                        warn!("Failed to send pong response");
                        break;
                    }
                } else {
                    // Echo other messages back
                    info!("Echoing message back");
                    if socket.send(Message::Text(text)).await.is_err() {
                        warn!("Failed to echo message");
                        break;
                    }
                }
            }
            Ok(Message::Binary(data)) => {
                info!("Received binary message ({} bytes)", data.len());
                // Echo binary messages back
                if socket.send(Message::Binary(data)).await.is_err() {
                    warn!("Failed to echo binary message");
                    break;
                }
            }
            Ok(Message::Close(_)) => {
                info!("Client sent close message");
                break;
            }
            Ok(Message::Ping(data)) => {
                info!("Received ping frame");
                if socket.send(Message::Pong(data)).await.is_err() {
                    warn!("Failed to send pong frame");
                    break;
                }
            }
            Ok(Message::Pong(_)) => {
                info!("Received pong frame");
            }
            Err(e) => {
                warn!("WebSocket error: {}", e);
                break;
            }
        }
    }

    info!("Client disconnected");
}

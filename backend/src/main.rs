use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};
use tracing_subscriber;
use uuid::Uuid;

// Message types for WebSocket communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Register { username: String },
    CreateRoom,
    JoinRoom { room_id: String },
    LeaveRoom,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    Registered { user_id: String },
    RoomCreated { room_id: String },
    RoomJoined { room_id: String, participants: Vec<String> },
    UserJoined { username: String },
    UserLeft { username: String },
    RoomLeft,
    Error { message: String },
    Pong,
}

// User info
#[derive(Debug, Clone)]
struct User {
    _id: String,
    username: String,
    tx: mpsc::UnboundedSender<String>,
}

// Room info
#[derive(Debug, Clone)]
struct Room {
    _id: String,
    participants: HashMap<String, String>, // user_id -> username
}

// Application state
type Rooms = Arc<RwLock<HashMap<String, Room>>>;
type Users = Arc<RwLock<HashMap<String, User>>>;
type UserRooms = Arc<RwLock<HashMap<String, String>>>; // user_id -> room_id

#[derive(Clone)]
struct AppState {
    rooms: Rooms,
    users: Users,
    user_rooms: UserRooms,
}

#[tokio::main]
async fn main() {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Create shared application state
    let state = AppState {
        rooms: Arc::new(RwLock::new(HashMap::new())),
        users: Arc::new(RwLock::new(HashMap::new())),
        user_rooms: Arc::new(RwLock::new(HashMap::new())),
    };

    // Build application router with WebSocket endpoint
    let app = Router::new()
        .route("/ws", get({
            let state = state.clone();
            move |ws| ws_handler(ws, state)
        }));

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
async fn ws_handler(ws: WebSocketUpgrade, state: AppState) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    info!("Client connected");

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let mut user_id: Option<String> = None;

    // Spawn a task to send messages to the client
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Process incoming messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    info!("Received message: {}", text);

                    // Parse message
                    let msg_result: Result<ClientMessage, _> = serde_json::from_str(&text);
                    
                    match msg_result {
                        Ok(client_msg) => {
                            match client_msg {
                                ClientMessage::Register { username } => {
                                    let new_user_id = Uuid::new_v4().to_string();
                                    info!("Registering user {} as {}", username, new_user_id);

                                    let user = User {
                                        _id: new_user_id.clone(),
                                        username: username.clone(),
                                        tx: tx.clone(),
                                    };

                                    state.users.write().await.insert(new_user_id.clone(), user);
                                    user_id = Some(new_user_id.clone());

                                    let response = ServerMessage::Registered {
                                        user_id: new_user_id,
                                    };
                                    let _ = tx.send(serde_json::to_string(&response).unwrap());
                                }
                                ClientMessage::CreateRoom => {
                                    if let Some(uid) = &user_id {
                                        let users = state.users.read().await;
                                        if let Some(user) = users.get(uid) {
                                            let username = user.username.clone();
                                            drop(users);

                                            let room_id = Uuid::new_v4().to_string();
                                            info!("User {} ({}) creating room {}", uid, username, room_id);

                                            let mut participants = HashMap::new();
                                            participants.insert(uid.clone(), username.clone());

                                            let room = Room {
                                                _id: room_id.clone(),
                                                participants,
                                            };

                                            state.rooms.write().await.insert(room_id.clone(), room);
                                            state.user_rooms.write().await.insert(uid.clone(), room_id.clone());

                                            let response = ServerMessage::RoomJoined {
                                                room_id,
                                                participants: vec![username],
                                            };
                                            let _ = tx.send(serde_json::to_string(&response).unwrap());
                                        }
                                    } else {
                                        let response = ServerMessage::Error {
                                            message: "Not registered".to_string(),
                                        };
                                        let _ = tx.send(serde_json::to_string(&response).unwrap());
                                    }
                                }
                                ClientMessage::JoinRoom { room_id } => {
                                    if let Some(uid) = &user_id {
                                        let users = state.users.read().await;
                                        if let Some(user) = users.get(uid) {
                                            let username = user.username.clone();
                                            drop(users);

                                            let mut rooms = state.rooms.write().await;
                                            if let Some(room) = rooms.get_mut(&room_id) {
                                                // Add user to room
                                                room.participants.insert(uid.clone(), username.clone());
                                                state.user_rooms.write().await.insert(uid.clone(), room_id.clone());

                                                let participants: Vec<String> = room.participants.values().cloned().collect();
                                                
                                                info!("User {} ({}) joined room {}", uid, username, room_id);

                                                // Send joined confirmation to the user
                                                let response = ServerMessage::RoomJoined {
                                                    room_id: room_id.clone(),
                                                    participants: participants.clone(),
                                                };
                                                let _ = tx.send(serde_json::to_string(&response).unwrap());

                                                // Notify other participants
                                                let notification = ServerMessage::UserJoined {
                                                    username: username.clone(),
                                                };
                                                let notification_str = serde_json::to_string(&notification).unwrap();

                                                let users_lock = state.users.read().await;
                                                for (participant_id, _) in &room.participants {
                                                    if participant_id != uid {
                                                        if let Some(participant) = users_lock.get(participant_id) {
                                                            let _ = participant.tx.send(notification_str.clone());
                                                        }
                                                    }
                                                }
                                            } else {
                                                let response = ServerMessage::Error {
                                                    message: "Room not found".to_string(),
                                                };
                                                let _ = tx.send(serde_json::to_string(&response).unwrap());
                                            }
                                        }
                                    } else {
                                        let response = ServerMessage::Error {
                                            message: "Not registered".to_string(),
                                        };
                                        let _ = tx.send(serde_json::to_string(&response).unwrap());
                                    }
                                }
                                ClientMessage::LeaveRoom => {
                                    if let Some(uid) = &user_id {
                                        let room_id_opt = state.user_rooms.write().await.remove(uid);
                                        
                                        if let Some(room_id) = room_id_opt {
                                            let mut rooms = state.rooms.write().await;
                                            if let Some(room) = rooms.get_mut(&room_id) {
                                                if let Some(username) = room.participants.remove(uid) {
                                                    info!("User {} ({}) left room {}", uid, username, room_id);

                                                    // Send confirmation to the user
                                                    let response = ServerMessage::RoomLeft;
                                                    let _ = tx.send(serde_json::to_string(&response).unwrap());

                                                    // Notify other participants
                                                    let notification = ServerMessage::UserLeft {
                                                        username: username.clone(),
                                                    };
                                                    let notification_str = serde_json::to_string(&notification).unwrap();

                                                    let users_lock = state.users.read().await;
                                                    for (participant_id, _) in &room.participants {
                                                        if let Some(participant) = users_lock.get(participant_id) {
                                                            let _ = participant.tx.send(notification_str.clone());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                ClientMessage::Ping => {
                                    let response = ServerMessage::Pong;
                                    let _ = tx.send(serde_json::to_string(&response).unwrap());
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse message: {}", e);
                            let response = ServerMessage::Error {
                                message: format!("Invalid message format: {}", e),
                            };
                            let _ = tx.send(serde_json::to_string(&response).unwrap());
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("Client sent close message");
                    break;
                }
                Ok(Message::Ping(_data)) => {
                    if tx.send(serde_json::to_string(&ServerMessage::Pong).unwrap()).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    warn!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        // Cleanup on disconnect
        if let Some(uid) = &user_id {
            info!("Cleaning up user {}", uid);

            // Remove from room if in one
            if let Some(room_id) = state.user_rooms.write().await.remove(uid) {
                let mut rooms = state.rooms.write().await;
                if let Some(room) = rooms.get_mut(&room_id) {
                    if let Some(username) = room.participants.remove(uid) {
                        info!("User {} left room {} on disconnect", username, room_id);

                        // Notify remaining participants
                        let notification = ServerMessage::UserLeft {
                            username,
                        };
                        let notification_str = serde_json::to_string(&notification).unwrap();

                        let users_lock = state.users.read().await;
                        for (participant_id, _) in &room.participants {
                            if let Some(participant) = users_lock.get(participant_id) {
                                let _ = participant.tx.send(notification_str.clone());
                            }
                        }
                    }
                }
            }

            // Remove user
            state.users.write().await.remove(uid);
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    info!("Client disconnected");
}

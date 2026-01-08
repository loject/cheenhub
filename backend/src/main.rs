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
use tracing::{info, warn, error};
use tracing_subscriber;
use uuid::Uuid;

mod sfu;
use sfu::SfuRouter;

// Participant information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParticipantInfo {
    username: String,
    user_id: String,
}

// Message types for WebSocket communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Register { username: String },
    CreateRoom,
    JoinRoom { room_id: String },
    LeaveRoom,
    Ping,
    // SFU-based WebRTC messages
    CreatePublisher,
    PublishAudio { sdp: String },
    CreateConsumer { publisher_user_id: String },
    ConsumerAnswer { consumer_id: String, sdp: String },
    PublisherIceCandidate { candidate: String },
    ConsumerIceCandidate { consumer_id: String, candidate: String },
    // Legacy P2P messages (deprecated, will be removed)
    WebrtcOffer { target_user_id: String, sdp: String },
    WebrtcAnswer { target_user_id: String, sdp: String },
    IceCandidate { target_user_id: String, candidate: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    Registered { user_id: String },
    RoomCreated { room_id: String },
    RoomJoined { room_id: String, participants: Vec<ParticipantInfo> },
    UserJoined { username: String, user_id: String },
    UserLeft { username: String, user_id: String },
    RoomLeft,
    Error { message: String },
    Pong,
    // SFU-based WebRTC messages
    PublisherCreated { sdp: String },
    AudioPublished { track_id: String },
    ConsumerCreated { consumer_id: String, publisher_user_id: String, sdp: String },
    NewPublisher { user_id: String, username: String },
    PublisherIceCandidate { candidate: String },
    ConsumerIceCandidate { consumer_id: String, candidate: String },
    // Legacy P2P messages (deprecated, will be removed)
    WebrtcOffer { from_user_id: String, sdp: String },
    WebrtcAnswer { from_user_id: String, sdp: String },
    IceCandidate { from_user_id: String, candidate: String },
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
    sfu_router: SfuRouter,
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
        sfu_router: SfuRouter::new(),
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
                    // Parse message
                    let msg_result: Result<ClientMessage, _> = serde_json::from_str(&text);
                    
                    match msg_result {
                        Ok(client_msg) => {
                            match client_msg {
                                ClientMessage::Register { username } => {
                                    let new_user_id = Uuid::new_v4().to_string();
                                    info!("[Room] Registering user {} as {}", username, new_user_id);

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
                                            info!("[Room] User {} ({}) creating room {}", uid, username, room_id);

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
                                                participants: vec![ParticipantInfo {
                                                    username,
                                                    user_id: uid.clone(),
                                                }],
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

                                                let participants: Vec<ParticipantInfo> = room.participants.iter()
                                                    .map(|(uid, uname)| ParticipantInfo {
                                                        username: uname.clone(),
                                                        user_id: uid.clone(),
                                                    })
                                                    .collect();
                                                
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
                                                    user_id: uid.clone(),
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
                                                        user_id: uid.clone(),
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
                                // SFU WebRTC handlers
                                ClientMessage::CreatePublisher => {
                                    if let Some(uid) = &user_id {
                                        let users = state.users.read().await;
                                        if let Some(user) = users.get(uid) {
                                            let username = user.username.clone();
                                            drop(users);

                                            info!("[SFU] Creating publisher for user {} ({})", username, uid);

                                            match state.sfu_router.add_publisher(uid.clone(), username.clone()).await {
                                                Ok(sdp_offer) => {
                                                    let response = ServerMessage::PublisherCreated {
                                                        sdp: sdp_offer,
                                                    };
                                                    let _ = tx.send(serde_json::to_string(&response).unwrap());

                                                    // NOTE: NewPublisher notification moved to PublishAudio handler
                                                    // to avoid race condition where consumers try to subscribe
                                                    // before the audio track is published
                                                }
                                                Err(e) => {
                                                    error!("[SFU] Failed to create publisher: {}", e);
                                                    let response = ServerMessage::Error {
                                                        message: format!("Failed to create publisher: {}", e),
                                                    };
                                                    let _ = tx.send(serde_json::to_string(&response).unwrap());
                                                }
                                            }
                                        }
                                    } else {
                                        let response = ServerMessage::Error {
                                            message: "Not registered".to_string(),
                                        };
                                        let _ = tx.send(serde_json::to_string(&response).unwrap());
                                    }
                                }
                                ClientMessage::PublishAudio { sdp } => {
                                    if let Some(uid) = &user_id {
                                        info!("[SFU] Setting publisher answer for user {}", uid);

                                        match state.sfu_router.set_publisher_answer(uid, sdp).await {
                                            Ok(track_id_opt) => {
                                                // Wait for track to be available
                                                let track_id = if track_id_opt.is_some() {
                                                    track_id_opt.unwrap()
                                                } else {
                                                    // Try to get track ID with retries
                                                    match state.sfu_router.get_publisher_track_id(uid, 50).await {
                                                        Some(tid) => tid,
                                                        None => {
                                                            warn!("[SFU] Track not available yet for user {}", uid);
                                                            "pending".to_string()
                                                        }
                                                    }
                                                };

                                                let response = ServerMessage::AudioPublished {
                                                    track_id,
                                                };
                                                let _ = tx.send(serde_json::to_string(&response).unwrap());

                                                // Now that audio track is published, notify other room members
                                                // This avoids race condition where consumers try to subscribe
                                                // before the audio track is available
                                                let users = state.users.read().await;
                                                if let Some(user) = users.get(uid) {
                                                    let username = user.username.clone();
                                                    drop(users);

                                                    if let Some(room_id) = state.user_rooms.read().await.get(uid) {
                                                        let rooms = state.rooms.read().await;
                                                        if let Some(room) = rooms.get(room_id) {
                                                            let notification = ServerMessage::NewPublisher {
                                                                user_id: uid.clone(),
                                                                username: username.clone(),
                                                            };
                                                            let notification_str = serde_json::to_string(&notification).unwrap();

                                                            let users_lock = state.users.read().await;
                                                            for (participant_id, _) in &room.participants {
                                                                if participant_id != uid {
                                                                    if let Some(participant) = users_lock.get(participant_id) {
                                                                        info!("[SFU] Notifying {} about new publisher {}", participant_id, uid);
                                                                        let _ = participant.tx.send(notification_str.clone());
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                error!("[SFU] Failed to set publisher answer: {}", e);
                                                let response = ServerMessage::Error {
                                                    message: format!("Failed to set publisher answer: {}", e),
                                                };
                                                let _ = tx.send(serde_json::to_string(&response).unwrap());
                                            }
                                        }
                                    }
                                }
                                ClientMessage::CreateConsumer { publisher_user_id } => {
                                    if let Some(uid) = &user_id {
                                        info!("[SFU] Creating consumer for user {} to consume {}", uid, publisher_user_id);

                                        match state.sfu_router.add_consumer(publisher_user_id.clone(), uid.clone()).await {
                                            Ok((consumer_id, sdp_offer)) => {
                                                let response = ServerMessage::ConsumerCreated {
                                                    consumer_id,
                                                    publisher_user_id: publisher_user_id.clone(),
                                                    sdp: sdp_offer,
                                                };
                                                let _ = tx.send(serde_json::to_string(&response).unwrap());
                                            }
                                            Err(e) => {
                                                error!("[SFU] Failed to create consumer: {}", e);
                                                let response = ServerMessage::Error {
                                                    message: format!("Failed to create consumer: {}", e),
                                                };
                                                let _ = tx.send(serde_json::to_string(&response).unwrap());
                                            }
                                        }
                                    }
                                }
                                ClientMessage::ConsumerAnswer { consumer_id, sdp } => {
                                    if let Some(_uid) = &user_id {
                                        info!("[SFU] Setting consumer answer for consumer {}", consumer_id);

                                        match state.sfu_router.set_consumer_answer(&consumer_id, sdp).await {
                                            Ok(_) => {
                                                info!("[SFU] Consumer {} answer set successfully", consumer_id);
                                            }
                                            Err(e) => {
                                                error!("[SFU] Failed to set consumer answer: {}", e);
                                                let response = ServerMessage::Error {
                                                    message: format!("Failed to set consumer answer: {}", e),
                                                };
                                                let _ = tx.send(serde_json::to_string(&response).unwrap());
                                            }
                                        }
                                    }
                                }
                                ClientMessage::PublisherIceCandidate { candidate } => {
                                    if let Some(uid) = &user_id {
                                        if let Err(e) = state.sfu_router.add_publisher_ice_candidate(uid, candidate).await {
                                            warn!("[SFU] Failed to add publisher ICE candidate: {}", e);
                                        }
                                    }
                                }
                                ClientMessage::ConsumerIceCandidate { consumer_id, candidate } => {
                                    if let Err(e) = state.sfu_router.add_consumer_ice_candidate(&consumer_id, candidate).await {
                                        warn!("[SFU] Failed to add consumer ICE candidate: {}", e);
                                    }
                                }
                                // Legacy WebRTC signaling relay logic (deprecated)
                                ClientMessage::WebrtcOffer { target_user_id, sdp } => {
                                    if let Some(uid) = &user_id {
                                        info!("Relaying WebRTC offer from {} to {}", uid, target_user_id);
                                        let users = state.users.read().await;
                                        if let Some(target_user) = users.get(&target_user_id) {
                                            let relay_msg = ServerMessage::WebrtcOffer {
                                                from_user_id: uid.clone(),
                                                sdp,
                                            };
                                            let _ = target_user.tx.send(serde_json::to_string(&relay_msg).unwrap());
                                        } else {
                                            let response = ServerMessage::Error {
                                                message: "Target user not found".to_string(),
                                            };
                                            let _ = tx.send(serde_json::to_string(&response).unwrap());
                                        }
                                    }
                                }
                                ClientMessage::WebrtcAnswer { target_user_id, sdp } => {
                                    if let Some(uid) = &user_id {
                                        info!("Relaying WebRTC answer from {} to {}", uid, target_user_id);
                                        let users = state.users.read().await;
                                        if let Some(target_user) = users.get(&target_user_id) {
                                            let relay_msg = ServerMessage::WebrtcAnswer {
                                                from_user_id: uid.clone(),
                                                sdp,
                                            };
                                            let _ = target_user.tx.send(serde_json::to_string(&relay_msg).unwrap());
                                        } else {
                                            let response = ServerMessage::Error {
                                                message: "Target user not found".to_string(),
                                            };
                                            let _ = tx.send(serde_json::to_string(&response).unwrap());
                                        }
                                    }
                                }
                                ClientMessage::IceCandidate { target_user_id, candidate } => {
                                    if let Some(uid) = &user_id {
                                        info!("Relaying ICE candidate from {} to {}", uid, target_user_id);
                                        let users = state.users.read().await;
                                        if let Some(target_user) = users.get(&target_user_id) {
                                            let relay_msg = ServerMessage::IceCandidate {
                                                from_user_id: uid.clone(),
                                                candidate,
                                            };
                                            let _ = target_user.tx.send(serde_json::to_string(&relay_msg).unwrap());
                                        } else {
                                            let response = ServerMessage::Error {
                                                message: "Target user not found".to_string(),
                                            };
                                            let _ = tx.send(serde_json::to_string(&response).unwrap());
                                        }
                                    }
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

            // Clean up SFU publisher and consumers
            if let Err(e) = state.sfu_router.remove_publisher(uid).await {
                warn!("[SFU] Failed to remove publisher during cleanup: {}", e);
            }
            if let Err(e) = state.sfu_router.remove_consumers_for_subscriber(uid).await {
                warn!("[SFU] Failed to remove consumers during cleanup: {}", e);
            }

            // Remove from room if in one
            if let Some(room_id) = state.user_rooms.write().await.remove(uid) {
                let mut rooms = state.rooms.write().await;
                if let Some(room) = rooms.get_mut(&room_id) {
                    if let Some(username) = room.participants.remove(uid) {
                        info!("User {} left room {} on disconnect", username, room_id);

                        // Notify remaining participants
                        let notification = ServerMessage::UserLeft {
                            username,
                            user_id: uid.clone(),
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

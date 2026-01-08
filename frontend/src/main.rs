use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    AudioContext, MessageEvent, MediaStream, UrlSearchParams, WebSocket,
    RtcPeerConnection, RtcConfiguration, RtcIceServer, RtcSessionDescriptionInit,
    RtcSdpType, RtcIceCandidateInit, RtcPeerConnectionIceEvent, RtcTrackEvent,
};
use js_sys::{Array, JsString, Reflect};

fn main() {
    console_error_panic_hook::set_once();

    // Initialize tracing for web console logging
    tracing_wasm::set_as_global_default_with_config(
        tracing_wasm::WASMLayerConfigBuilder::new()
            .set_max_level(tracing::Level::INFO)
            .build()
    );
    
    dioxus::launch(App);
}

// Participant information structure (matching backend)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParticipantInfo {
    username: String,
    user_id: String,
}

// Message types matching backend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Register { username: String },
    CreateRoom,
    JoinRoom { room_id: String },
    LeaveRoom,
    Ping,
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
    WebrtcOffer { from_user_id: String, sdp: String },
    WebrtcAnswer { from_user_id: String, sdp: String },
    IceCandidate { from_user_id: String, candidate: String },
}

// Microphone status enum
#[derive(Clone, PartialEq)]
enum MicStatus {
    NotRequested,
    Requesting,
    Allowed,
    Denied,
}

impl std::fmt::Display for MicStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MicStatus::NotRequested => write!(f, "Not requested"),
            MicStatus::Requesting => write!(f, "Requesting..."),
            MicStatus::Allowed => write!(f, "Allowed ✓"),
            MicStatus::Denied => write!(f, "Denied ✗"),
        }
    }
}

// Participant info with user_id
#[derive(Clone, Debug)]
struct Participant {
    username: String,
    user_id: String,
}

#[component]
fn App() -> Element {
    // State for username input
    let mut username = use_signal(|| String::from(""));
    
    // State for connection status
    let mut status = use_signal(|| "Disconnected".to_string());
    
    // State to hold the WebSocket connection
    let mut ws = use_signal(|| None::<WebSocket>);
    
    // State for microphone
    let mut mic_status = use_signal(|| MicStatus::NotRequested);
    let mut media_stream = use_signal(|| None::<MediaStream>);
    let audio_level = use_signal(|| 0.0);
    
    // State for rooms
    let mut user_id = use_signal(|| None::<String>);
    let mut current_room = use_signal(|| None::<String>);
    let mut room_input = use_signal(|| String::from(""));
    let mut participants = use_signal(|| Vec::<Participant>::new());
    
    // TODO: Replace Mesh topology with SFU for better scalability
    // WebRTC state - peer connections per user
    let mut peer_connections = use_signal(|| HashMap::<String, RtcPeerConnection>::new());
    
    // TODO: Move media processing to SFU server
    // Audio levels for each participant
    let mut participant_audio_levels = use_signal(|| HashMap::<String, f64>::new());
    
    // Check URL for room parameter on mount
    use_effect(move || {
        let window = web_sys::window().expect("no window");
        let location = window.location();
        let search = location.search().unwrap_or_default();
        
        if !search.is_empty() {
            if let Ok(params) = UrlSearchParams::new_with_str(&search) {
                if let Some(room_id) = params.get("room") {
                    info!("Found room in URL: {}", room_id);
                    room_input.set(room_id);
                }
            }
        }
    });
    
    // Get WebSocket URL (use current host)
    let get_ws_url = || {
        let window = web_sys::window().expect("no window");
        let location = window.location();
        let protocol = if location.protocol().unwrap_or_default() == "https:" {
            "wss:"
        } else {
            "ws:"
        };
        let host = location.host().unwrap_or_else(|_| "localhost:8080".to_string());
        format!("{}//{}:8080/ws", protocol, host.split(':').next().unwrap_or("localhost"))
    };
    
    // Get share URL
    let get_share_url = move |room_id: &str| {
        let window = web_sys::window().expect("no window");
        let location = window.location();
        let protocol = location.protocol().unwrap_or_default();
        let host = location.host().unwrap_or_else(|_| "localhost:8080".to_string());
        format!("{}//{}?room={}", protocol, host, room_id)
    };

    // Handler for connecting to the server
    let connect = move |_| {
        let username_val = username.read().clone();
        
        if username_val.is_empty() {
            info!("Username is empty, not connecting");
            return;
        }

        info!("Attempting to connect to WebSocket server...");
        
        let ws_url = get_ws_url();
        info!("Connecting to: {}", ws_url);
        
        // Create WebSocket connection
        match WebSocket::new(&ws_url) {
            Ok(websocket) => {
                info!("WebSocket created successfully");
                
                // Clone for closures
                let ws_clone = websocket.clone();
                let username_clone = username_val.clone();
                
                // Set up onopen handler
                let onopen = Closure::wrap(Box::new(move |_| {
                    info!("WebSocket connection opened");
                    status.set("Connected".to_string());
                    
                    // Register user
                    let register_msg = ClientMessage::Register {
                        username: username_clone.clone(),
                    };
                    let msg_str = serde_json::to_string(&register_msg).unwrap();
                    if let Err(e) = ws_clone.send_with_str(&msg_str) {
                        info!("Failed to send register: {:?}", e);
                    } else {
                        info!("Sent register message");
                    }
                }) as Box<dyn FnMut(JsValue)>);
                
                websocket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
                onopen.forget();
                
                // Set up onmessage handler
                let ws_for_msg = websocket.clone();
                let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                    if let Ok(txt) = e.data().dyn_into::<JsString>() {
                        let message: String = txt.into();
                        
                        // Parse server message
                        if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&message) {
                            match server_msg {
                                ServerMessage::Registered { user_id: uid } => {
                                    info!("[Room] Registered with user_id: {}", uid);
                                    user_id.set(Some(uid));
                                    
                                    // Auto-join room if room_id is present in URL
                                    let room_id_val = room_input.read().clone();
                                    if !room_id_val.is_empty() {
                                        info!("Auto-joining room: {}", room_id_val);
                                        let join_msg = ClientMessage::JoinRoom { room_id: room_id_val };
                                        if let Ok(msg_str) = serde_json::to_string(&join_msg) {
                                            if let Err(e) = ws_for_msg.send_with_str(&msg_str) {
                                                info!("Failed to auto-join room: {:?}", e);
                                            }
                                        }
                                    }
                                }
                                ServerMessage::RoomCreated { room_id: rid } => {
                                    info!("[Room] Room created: {}", rid);
                                    current_room.set(Some(rid.clone()));
                                    room_input.set(rid);
                                    participants.set(vec![]);
                                }
                                ServerMessage::RoomJoined { room_id: rid, participants: parts_info } => {
                                    info!("[Room] Joined room: {}", rid);
                                    current_room.set(Some(rid));
                                    // Convert ParticipantInfo to Participant
                                    let parts: Vec<Participant> = parts_info.into_iter()
                                        .map(|info| Participant {
                                            username: info.username,
                                            user_id: info.user_id,
                                        })
                                        .collect();
                                    
                                    info!("[Room] Received {} participants with user_ids", parts.len());
                                    for p in &parts {
                                        info!("[Room] Participant: {} (user_id: {})", p.username, p.user_id);
                                    }
                                    
                                    participants.set(parts);
                                }
                                ServerMessage::UserJoined { username, user_id } => {
                                    info!("[Room] User joined: {} ({})", username, user_id);
                                    
                                    // Безопасное добавление участника
                                    participants.write().push(Participant {
                                        username: username.clone(),
                                        user_id: user_id.clone(),
                                    });
                                    
                                    // Проверить что у нас есть микрофон перед созданием peer connection
                                    let stream = match media_stream.read().as_ref() {
                                        Some(s) => {
                                            info!("[WebRTC] Microphone available, creating peer connection for {}", username);
                                            s.clone()
                                        }
                                        None => {
                                            info!("[WebRTC] No microphone yet, skipping peer connection for {}", username);
                                            return;
                                        }
                                    };
                                    
                                    // WebSocket уже доступен (мы внутри onmessage handler)
                                    info!("[WebRTC] Initiating peer connection for {} ({})", username, user_id);
                                    
                                    // Клонировать переменные с логированием каждого шага
                                    info!("[WebRTC] Step 0.1: About to clone stream");
                                    let stream_clone = stream.clone();
                                    info!("[WebRTC] Step 0.2: Stream cloned successfully");
                                    
                                    info!("[WebRTC] Step 0.3: About to clone user_id");
                                    let target_uid = user_id.clone();
                                    info!("[WebRTC] Step 0.4: user_id cloned successfully");
                                    
                                    info!("[WebRTC] Step 0.5: About to clone username");
                                    let target_name = username.clone();
                                    info!("[WebRTC] Step 0.6: username cloned successfully");
                                    
                                    info!("[WebRTC] Step 0.7: About to clone WebSocket");
                                    let ws_clone = ws_for_msg.clone();
                                    info!("[WebRTC] Step 0.8: WebSocket cloned successfully");
                                    
                                    info!("[WebRTC] Step 0.9: All variables cloned, about to spawn task for {}", target_uid);
                                    
                                    // Создать peer connection безопасно
                                    spawn_local(async move {
                                        info!("[WebRTC] INSIDE SPAWN_LOCAL - VERY FIRST LINE - Starting task");
                                        info!("[WebRTC] INSIDE SPAWN_LOCAL - Step 1: Starting spawn for {} ({})", target_name, target_uid);
                                        info!("[WebRTC] Step 2: About to call create_peer_connection");
                                        
                                        match create_peer_connection(stream_clone, target_uid.clone(), ws_clone, true, participant_audio_levels).await {
                                            Ok(pc) => {
                                                info!("[WebRTC] Step 3: create_peer_connection succeeded for {} ({})", target_name, target_uid);
                                                info!("[WebRTC] Step 4: Inserting peer connection into map");
                                                peer_connections.write().insert(target_uid.clone(), pc);
                                                info!("[WebRTC] Step 5: Peer connection stored successfully for {}", target_uid);
                                            }
                                            Err(e) => {
                                                info!("[Error] create_peer_connection failed for {} ({}): {:?}", target_name, target_uid, e);
                                            }
                                        }
                                        info!("[WebRTC] Step 6: Spawn block completed for {}", target_name);
                                    });
                                    
                                    info!("[WebRTC] Step 0.10: Spawn created successfully");
                                }
                                ServerMessage::UserLeft { username: uname, user_id: uid } => {
                                    info!("[Room] User left: {} ({})", uname, uid);
                                    participants.write().retain(|p| p.user_id != uid);
                                    
                                    // Close and remove peer connection
                                    if let Some(pc) = peer_connections.write().remove(&uid) {
                                        pc.close();
                                    }
                                    participant_audio_levels.write().remove(&uid);
                                }
                                ServerMessage::RoomLeft => {
                                    info!("[Room] Left room");
                                    current_room.set(None);
                                    participants.set(vec![]);
                                    
                                    // Close all peer connections
                                    for (_, pc) in peer_connections.write().drain() {
                                        pc.close();
                                    }
                                    participant_audio_levels.write().clear();
                                }
                                ServerMessage::Error { message: err } => {
                                    info!("[Error] Server error: {}", err);
                                }
                                ServerMessage::Pong => {
                                    // Pong received - no logging needed
                                }
                                ServerMessage::WebrtcOffer { from_user_id, sdp } => {
                                    info!("[WebRTC] Received offer from {}", from_user_id);
                                    info!("[DEBUG] About to check media_stream for offer handling");
                                    
                                    if let Some(stream) = media_stream.read().as_ref() {
                                        info!("[DEBUG] Media stream found, spawning offer handler");
                                        spawn_local({
                                            let stream = stream.clone();
                                            let from_uid = from_user_id.clone();
                                            let ws = ws_for_msg.clone();
                                            let offer_sdp = sdp.clone();
                                            async move {
                                                info!("[DEBUG] INSIDE SPAWN - offer handler started for {}", from_uid);
                                                info!("[DEBUG] About to call handle_webrtc_offer");
                                                match handle_webrtc_offer(stream, from_uid.clone(), ws, offer_sdp, participant_audio_levels).await {
                                                    Ok(pc) => {
                                                        info!("[DEBUG] handle_webrtc_offer succeeded, about to write to peer_connections");
                                                        peer_connections.write().insert(from_uid, pc);
                                                        info!("[DEBUG] peer_connection inserted successfully");
                                                    }
                                                    Err(e) => {
                                                        info!("Failed to handle WebRTC offer: {:?}", e);
                                                    }
                                                }
                                            }
                                        });
                                    }
                                }
                                ServerMessage::WebrtcAnswer { from_user_id, sdp } => {
                                    info!("Received WebRTC answer from {}", from_user_id);
                                    info!("[DEBUG] About to read peer_connections for answer handling");
                                    
                                    if let Some(pc) = peer_connections.read().get(&from_user_id) {
                                        info!("[DEBUG] Found peer connection for {}, spawning answer handler", from_user_id);
                                        spawn_local({
                                            let pc = pc.clone();
                                            let answer_sdp = sdp.clone();
                                            let from_uid_debug = from_user_id.clone();
                                            async move {
                                                info!("[DEBUG] INSIDE SPAWN - answer handler started for {}", from_uid_debug);
                                                if let Err(e) = handle_webrtc_answer(pc, answer_sdp).await {
                                                    info!("Failed to handle WebRTC answer: {:?}", e);
                                                }
                                            }
                                        });
                                    }
                                }
                                ServerMessage::IceCandidate { from_user_id, candidate } => {
                                    info!("Received ICE candidate from {}", from_user_id);
                                    info!("[DEBUG] About to read peer_connections for ICE candidate - THIS IS LINE 384");
                                    
                                    if let Some(pc) = peer_connections.read().get(&from_user_id) {
                                        info!("[DEBUG] Found peer connection for {}, spawning ICE handler", from_user_id);
                                        spawn_local({
                                            let pc = pc.clone();
                                            let cand = candidate.clone();
                                            let from_uid_debug = from_user_id.clone();
                                            async move {
                                                info!("[DEBUG] INSIDE SPAWN_LOCAL - ICE handler started for {}", from_uid_debug);
                                                if let Err(e) = handle_ice_candidate(pc, cand).await {
                                                    info!("Failed to handle ICE candidate: {:?}", e);
                                                }
                                            }
                                        });
                                    } else {
                                        info!("[DEBUG] No peer connection found for {} when handling ICE candidate", from_user_id);
                                    }
                                }
                            }
                        }
                    }
                }) as Box<dyn FnMut(MessageEvent)>);
                
                websocket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
                onmessage.forget();
                
                // Set up onerror handler
                let onerror = Closure::wrap(Box::new(move |e: JsValue| {
                    info!("WebSocket error: {:?}", e);
                    status.set("Disconnected (Error)".to_string());
                }) as Box<dyn FnMut(JsValue)>);
                
                websocket.set_onerror(Some(onerror.as_ref().unchecked_ref()));
                onerror.forget();
                
                // Set up onclose handler
                let onclose = Closure::wrap(Box::new(move |_| {
                    info!("WebSocket connection closed");
                    status.set("Disconnected".to_string());
                    user_id.set(None);
                    current_room.set(None);
                    participants.set(vec![]);
                    
                    // Close all peer connections
                    for (_, pc) in peer_connections.write().drain() {
                        pc.close();
                    }
                    participant_audio_levels.write().clear();
                }) as Box<dyn FnMut(JsValue)>);
                
                websocket.set_onclose(Some(onclose.as_ref().unchecked_ref()));
                onclose.forget();
                
                // Store the WebSocket connection
                ws.set(Some(websocket));
            }
            Err(e) => {
                info!("Failed to create WebSocket: {:?}", e);
                status.set("Connection Failed".to_string());
            }
        }
    };
    
    // Handler for creating room
    let create_room = move |_| {
        if let Some(websocket) = ws.read().as_ref() {
            let msg = ClientMessage::CreateRoom;
            let msg_str = serde_json::to_string(&msg).unwrap();
            let _ = websocket.send_with_str(&msg_str);
        }
    };
    
    // Handler for joining room
    let join_room = move |_| {
        if let Some(websocket) = ws.read().as_ref() {
            let room_id = room_input.read().clone();
            if !room_id.is_empty() {
                let msg = ClientMessage::JoinRoom { room_id };
                let msg_str = serde_json::to_string(&msg).unwrap();
                let _ = websocket.send_with_str(&msg_str);
            }
        }
    };
    
    // Handler for leaving room
    let leave_room = move |_| {
        if let Some(websocket) = ws.read().as_ref() {
            let msg = ClientMessage::LeaveRoom;
            let msg_str = serde_json::to_string(&msg).unwrap();
            let _ = websocket.send_with_str(&msg_str);
        }
    };
    
    // Handler for copying room link
    let copy_link = move |_| {
        if let Some(room_id) = current_room.read().as_ref() {
            let share_url = get_share_url(room_id);
            
            // Copy to clipboard using the Clipboard API
            if let Some(window) = web_sys::window() {
                let navigator = window.navigator();
                let clipboard = navigator.clipboard();
                let _ = clipboard.write_text(&share_url);
                info!("Copied to clipboard: {}", share_url);
            }
        }
    };
    
    // Handler for requesting microphone access
    let request_microphone = move |_| {
        mic_status.set(MicStatus::Requesting);
        info!("Requesting microphone access...");
        
        spawn_local(async move {
            let window = match web_sys::window() {
                Some(w) => w,
                None => {
                    info!("[Error] No global window available");
                    mic_status.set(MicStatus::Denied);
                    return;
                }
            };
            
            let navigator = window.navigator();
            
            let media_devices = match navigator.media_devices() {
                Ok(md) => md,
                Err(e) => {
                    info!("[Error] No media devices available: {:?}", e);
                    mic_status.set(MicStatus::Denied);
                    return;
                }
            };
            
            // Create constraints for audio only
            let constraints = web_sys::MediaStreamConstraints::new();
            constraints.set_audio(&JsValue::from(true));
            constraints.set_video(&JsValue::from(false));
            
            let get_user_media_promise = match media_devices.get_user_media_with_constraints(&constraints) {
                Ok(promise) => promise,
                Err(e) => {
                    info!("[Error] Failed to call getUserMedia: {:?}", e);
                    mic_status.set(MicStatus::Denied);
                    return;
                }
            };
            
            match wasm_bindgen_futures::JsFuture::from(get_user_media_promise).await {
                Ok(stream_val) => {
                    info!("Microphone access granted");
                    
                    let stream: MediaStream = match stream_val.dyn_into() {
                        Ok(s) => s,
                        Err(e) => {
                            info!("[Error] Failed to convert to MediaStream: {:?}", e);
                            mic_status.set(MicStatus::Denied);
                            return;
                        }
                    };
                    
                    // Start audio analysis
                    start_audio_analysis(stream.clone(), audio_level);
                    
                    media_stream.set(Some(stream.clone()));
                    mic_status.set(MicStatus::Allowed);
                    
                    // If we're already in a room - create peer connections for all participants
                    info!("[WebRTC] Checking if we're in a room for deferred connections...");
                    
                    let in_room = current_room.read().is_some();
                    if !in_room {
                        info!("[WebRTC] Not in a room, skipping deferred connections");
                    } else {
                        info!("[WebRTC] Microphone obtained while in room, initiating connections");
                        
                        // Safely get user_id
                        info!("[WebRTC] Getting current user_id...");
                        let current_uid = match user_id.read().as_ref() {
                            Some(id) => {
                                info!("[WebRTC] Current user_id: {}", id);
                                id.clone()
                            }
                            None => {
                                info!("[Error] No user_id set, cannot create deferred connections");
                                return; // Early return from async block
                            }
                        };
                        
                        // Safely get WebSocket connection
                        info!("[WebRTC] Getting WebSocket connection...");
                        let ws_sock = match ws.read().as_ref() {
                            Some(socket) => {
                                info!("[WebRTC] WebSocket connection available");
                                socket.clone()
                            }
                            None => {
                                info!("[Error] No WebSocket connection, cannot create deferred connections");
                                return; // Early return from async block
                            }
                        };
                        
                        // Safely get participants list
                        info!("[WebRTC] Getting participants list...");
                        let parts = participants.read().clone();
                        info!("[WebRTC] Found {} participants", parts.len());
                        
                        // Iterate through participants safely
                        for (idx, participant) in parts.iter().enumerate() {
                            info!("[WebRTC] Processing participant {}/{}: {} (user_id: {})",
                                idx + 1, parts.len(), participant.username, participant.user_id);
                            
                            // Skip if user_id is empty
                            if participant.user_id.is_empty() {
                                info!("[WebRTC] Skipping participant {} - empty user_id", participant.username);
                                continue;
                            }
                            
                            // Skip if this is us
                            if participant.user_id == current_uid {
                                info!("[WebRTC] Skipping participant {} - this is us", participant.username);
                                continue;
                            }
                            
                            // Skip if peer connection already exists
                            let has_connection = peer_connections.read().contains_key(&participant.user_id);
                            if has_connection {
                                info!("[WebRTC] Peer connection already exists for {} ({})",
                                    participant.username, participant.user_id);
                                continue;
                            }
                            
                            // Create peer connection for this participant
                            info!("[WebRTC] Creating peer connection for existing participant: {} ({})",
                                participant.username, participant.user_id);
                            
                            // Clone necessary data for spawn
                            let stream_clone = stream.clone();
                            let target_uid = participant.user_id.clone();
                            let ws_clone = ws_sock.clone();
                            let participant_name = participant.username.clone();
                            
                            info!("[WebRTC] Spawning connection task for {}", target_uid);
                            
                            // Clone target_uid again for use after spawn
                            let target_uid_for_log = target_uid.clone();
                            
                            spawn_local(async move {
                                info!("[WebRTC] Starting peer connection creation for {} in spawned task", target_uid);
                                
                                match create_peer_connection(
                                    stream_clone,
                                    target_uid.clone(),
                                    ws_clone,
                                    true,
                                    participant_audio_levels
                                ).await {
                                    Ok(pc) => {
                                        info!("[WebRTC] Successfully created peer connection for {} ({})",
                                            participant_name, target_uid);
                                        peer_connections.write().insert(target_uid, pc);
                                    }
                                    Err(e) => {
                                        info!("[Error] Failed to create peer connection for {} ({}): {:?}",
                                            participant_name, target_uid, e);
                                    }
                                }
                            });
                            
                            info!("[WebRTC] Successfully spawned connection task for {}", target_uid_for_log);
                        }
                        
                        info!("[WebRTC] Finished processing all participants for deferred connections");
                    }
                }
                Err(e) => {
                    info!("Microphone access denied: {:?}", e);
                    mic_status.set(MicStatus::Denied);
                }
            }
        });
    };

    rsx! {
        style { {include_str!("../style.css")} }
        
        div { class: "container",
            h1 { "Voice Messenger PoC" }
            
            div { class: "status-bar",
                span { "Server: " }
                span { 
                    class: if status.read().starts_with("Connected") { "status-connected" } else { "status-disconnected" },
                    "{status}"
                }
            }
            
            div { class: "status-bar mic-status",
                span { "Microphone: " }
                span {
                    class: match *mic_status.read() {
                        MicStatus::Allowed => "status-connected",
                        MicStatus::Denied => "status-disconnected",
                        MicStatus::Requesting => "status-requesting",
                        MicStatus::NotRequested => "",
                    },
                    "{mic_status}"
                }
            }
            
            // Audio level indicator
            if *mic_status.read() == MicStatus::Allowed {
                div { class: "audio-meter",
                    div { class: "audio-meter-label", "Audio Level:" }
                    div { class: "audio-meter-bar",
                        div { 
                            class: "audio-meter-fill",
                            style: "width: {audio_level}%"
                        }
                    }
                }
            }
            
            div { class: "form-group",
                label { r#for: "username", "Username:" }
                input {
                    id: "username",
                    r#type: "text",
                    value: "{username}",
                    placeholder: "Enter your username",
                    oninput: move |evt| username.set(evt.value().clone()),
                    disabled: ws.read().is_some(),
                }
            }
            
            button {
                class: "connect-btn",
                onclick: connect,
                disabled: ws.read().is_some(),
                "Connect to Server"
            }
            
            button {
                class: "mic-btn",
                onclick: request_microphone,
                disabled: *mic_status.read() != MicStatus::NotRequested,
                "Request Microphone Access"
            }
            
            // Room management section
            if ws.read().is_some() && user_id.read().is_some() {
                div { class: "room-section",
                    h2 { "Room Management" }
                    
                    if current_room.read().is_none() {
                        // Not in a room
                        div { class: "room-controls",
                            button {
                                class: "room-btn",
                                onclick: create_room,
                                "Create New Room"
                            }
                            
                            div { class: "form-group",
                                label { "Or join existing room:" }
                                div { class: "join-room-input",
                                    input {
                                        r#type: "text",
                                        value: "{room_input}",
                                        placeholder: "Enter room ID",
                                        oninput: move |evt| room_input.set(evt.value().clone())
                                    }
                                    button {
                                        class: "join-btn",
                                        onclick: join_room,
                                        disabled: room_input.read().is_empty(),
                                        "Join Room"
                                    }
                                }
                            }
                        }
                    } else {
                        // In a room
                        div { class: "room-info",
                            div { class: "room-id-section",
                                h3 { "Room: {current_room.read().as_ref().unwrap()}" }
                                button {
                                    class: "copy-btn",
                                    onclick: copy_link,
                                    "📋 Copy Room Link"
                                }
                            }
                            
                            div { class: "participants-section",
                                h4 { "Participants ({participants.read().len()}):" }
                                ul { class: "participants-list",
                                    for participant in participants.read().iter() {
                                        li { 
                                            class: "participant-item",
                                            span { class: "participant-name", "{participant.username}" }
                                            // Show compact audio meter for each participant
                                            if !participant.user_id.is_empty() {
                                                {
                                                    let level = participant_audio_levels.read()
                                                        .get(&participant.user_id)
                                                        .copied()
                                                        .unwrap_or(0.0);
                                                    rsx! {
                                                        div { class: "participant-audio-meter",
                                                            div { 
                                                                class: "participant-audio-fill",
                                                                style: "width: {level}%"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            
                            button {
                                class: "leave-btn",
                                onclick: leave_room,
                                "Leave Room"
                            }
                        }
                    }
                }
            }
            
            div { class: "info",
                p { "Instructions:" }
                ul {
                    li { "Enter your username and click 'Connect to Server'" }
                    li { "Request microphone access to enable voice" }
                    li { "Create a new room or join an existing one" }
                    li { "Share the room link with others to invite them" }
                    li { "Audio levels shown for each participant" }
                    li { "Check browser console for detailed logs" }
                }
            }
        }
    }
}

// Function to start audio analysis and update audio level
fn start_audio_analysis(stream: MediaStream, mut audio_level: Signal<f64>) {
    spawn_local(async move {
        let audio_context = match AudioContext::new() {
            Ok(ctx) => ctx,
            Err(e) => {
                info!("[Error] Failed to create AudioContext: {:?}", e);
                return;
            }
        };
        
        let source = match audio_context.create_media_stream_source(&stream) {
            Ok(s) => s,
            Err(e) => {
                info!("[Error] Failed to create media stream source: {:?}", e);
                return;
            }
        };
        
        let analyser = match audio_context.create_analyser() {
            Ok(a) => a,
            Err(e) => {
                info!("[Error] Failed to create analyser: {:?}", e);
                return;
            }
        };
        analyser.set_fft_size(2048);
        
        if let Err(e) = source.connect_with_audio_node(&analyser) {
            info!("[Error] Failed to connect source to analyser: {:?}", e);
            return;
        }
        
        let buffer_length = analyser.frequency_bin_count();
        
        // Use setInterval instead of requestAnimationFrame for simplicity
        let window = match web_sys::window() {
            Some(w) => w,
            None => {
                info!("[Error] No window available for audio analysis");
                return;
            }
        };
        
        let closure = Closure::wrap(Box::new(move || {
            let mut data_array = vec![0u8; buffer_length as usize];
            analyser.get_byte_time_domain_data(&mut data_array);
            
            // Calculate RMS (Root Mean Square) for audio level
            let mut sum = 0.0;
            for &value in data_array.iter() {
                let normalized = value as f64 - 128.0;
                sum += normalized * normalized;
            }
            let rms = (sum / buffer_length as f64).sqrt();
            
            // Normalize to 0-100 range (typical speech is around 10-30, normalize to make it more visible)
            let level = (rms / 30.0 * 100.0).min(100.0);
            audio_level.set(level);
        }) as Box<dyn FnMut()>);
        
        // Update every 50ms (20 times per second)
        match window.set_interval_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            50
        ) {
            Ok(_) => {
                info!("[Audio] Started local audio level monitoring");
            }
            Err(e) => {
                info!("[Error] Failed to set interval for audio monitoring: {:?}", e);
                return;
            }
        }
        
        // Keep closure alive
        closure.forget();
    });
}

// Function to start audio analysis for remote stream and update participant audio level
fn start_remote_audio_analysis(stream: MediaStream, user_id: String, mut participant_audio_levels: Signal<HashMap<String, f64>>) {
    spawn_local(async move {
        let audio_context = match AudioContext::new() {
            Ok(ctx) => ctx,
            Err(e) => {
                info!("[Error] Failed to create AudioContext for remote stream {}: {:?}", user_id, e);
                return;
            }
        };
        
        let source = match audio_context.create_media_stream_source(&stream) {
            Ok(s) => s,
            Err(e) => {
                info!("[Error] Failed to create media stream source for {}: {:?}", user_id, e);
                return;
            }
        };
        
        let analyser = match audio_context.create_analyser() {
            Ok(a) => a,
            Err(e) => {
                info!("[Error] Failed to create analyser for {}: {:?}", user_id, e);
                return;
            }
        };
        analyser.set_fft_size(2048);
        
        if let Err(e) = source.connect_with_audio_node(&analyser) {
            info!("[Error] Failed to connect source to analyser for {}: {:?}", user_id, e);
            return;
        }
        
        let buffer_length = analyser.frequency_bin_count();
        
        let window = match web_sys::window() {
            Some(w) => w,
            None => {
                info!("[Error] No window available for remote audio analysis of {}", user_id);
                return;
            }
        };
        
        let uid_clone = user_id.clone();
        let closure = Closure::wrap(Box::new(move || {
            let mut data_array = vec![0u8; buffer_length as usize];
            analyser.get_byte_time_domain_data(&mut data_array);
            
            // Calculate RMS
            let mut sum = 0.0;
            for &value in data_array.iter() {
                let normalized = value as f64 - 128.0;
                sum += normalized * normalized;
            }
            let rms = (sum / buffer_length as f64).sqrt();
            
            let level = (rms / 30.0 * 100.0).min(100.0);
            participant_audio_levels.write().insert(uid_clone.clone(), level);
        }) as Box<dyn FnMut()>);
        
        match window.set_interval_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            50
        ) {
            Ok(_) => {
                info!("[Audio] Started remote audio level monitoring for {}", user_id);
            }
            Err(e) => {
                info!("[Error] Failed to set interval for remote audio monitoring {}: {:?}", user_id, e);
                return;
            }
        }
        
        closure.forget();
    });
}

// Create RTCPeerConnection with ICE servers
fn create_rtc_peer_connection() -> Result<RtcPeerConnection, JsValue> {
    let mut config = RtcConfiguration::new();
    
    // Add STUN servers (using Google's public STUN servers)
    let ice_servers = Array::new();
    let stun_server = RtcIceServer::new();
    stun_server.set_urls(&JsValue::from_str("stun:stun.l.google.com:19302"));
    ice_servers.push(&stun_server);
    
    let stun_server2 = RtcIceServer::new();
    stun_server2.set_urls(&JsValue::from_str("stun:stun1.l.google.com:19302"));
    ice_servers.push(&stun_server2);
    
    config.set_ice_servers(&ice_servers);
    
    RtcPeerConnection::new_with_configuration(&config)
}

// Create peer connection and optionally create offer
async fn create_peer_connection(
    local_stream: MediaStream,
    target_user_id: String,
    ws: WebSocket,
    create_offer: bool,
    participant_audio_levels: Signal<HashMap<String, f64>>,
) -> Result<RtcPeerConnection, JsValue> {
    info!("Creating peer connection for user {}", target_user_id);
    
    let pc = create_rtc_peer_connection()?;
    
    // Add local tracks to peer connection
    let tracks = local_stream.get_tracks();
    for i in 0..tracks.length() {
        if let Some(track) = tracks.get(i).dyn_into::<web_sys::MediaStreamTrack>().ok() {
            let streams = Array::new();
            streams.push(&local_stream);
            let _ = pc.add_track(&track, &local_stream, &streams);
        }
    }
    
    // Set up onicecandidate handler
    let ws_clone = ws.clone();
    let target_uid = target_user_id.clone();
    let onicecandidate = Closure::wrap(Box::new(move |ev: RtcPeerConnectionIceEvent| {
        if let Some(candidate) = ev.candidate() {
            info!("ICE candidate generated for {}", target_uid);
            let candidate_json = candidate.to_json();
            
            // Extract candidate string
            if let Ok(candidate_str) = Reflect::get(&candidate_json, &JsValue::from_str("candidate")) {
                if let Some(cand_str) = candidate_str.as_string() {
                    let msg = ClientMessage::IceCandidate {
                        target_user_id: target_uid.clone(),
                        candidate: cand_str,
                    };
                    if let Ok(msg_str) = serde_json::to_string(&msg) {
                        let _ = ws_clone.send_with_str(&msg_str);
                    }
                }
            }
        }
    }) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>);
    
    pc.set_onicecandidate(Some(onicecandidate.as_ref().unchecked_ref()));
    onicecandidate.forget();
    
    // Set up ontrack handler to receive remote audio
    let target_uid_track = target_user_id.clone();
    let ontrack = Closure::wrap(Box::new(move |ev: RtcTrackEvent| {
        info!("Received remote track from {}", target_uid_track);
        
        let streams = ev.streams();
        if streams.length() > 0 {
            if let Some(remote_stream) = streams.get(0).dyn_into::<MediaStream>().ok() {
                // Play the remote audio stream - use safe error handling
                match web_sys::HtmlAudioElement::new() {
                    Ok(audio) => {
                        audio.set_src_object(Some(&remote_stream));
                        audio.set_autoplay(true);
                        match audio.play() {
                            Ok(_) => {
                                info!("[Audio] Started playing remote audio from {}", target_uid_track);
                            }
                            Err(e) => {
                                info!("[Error] Failed to play remote audio from {}: {:?}", target_uid_track, e);
                            }
                        }
                        
                        // Start audio analysis for this remote stream
                        start_remote_audio_analysis(remote_stream, target_uid_track.clone(), participant_audio_levels);
                    }
                    Err(e) => {
                        info!("[Error] Failed to create audio element for {}: {:?}", target_uid_track, e);
                    }
                }
            }
        }
    }) as Box<dyn FnMut(RtcTrackEvent)>);
    
    pc.set_ontrack(Some(ontrack.as_ref().unchecked_ref()));
    ontrack.forget();
    
    // Create offer if requested
    if create_offer {
        info!("Creating offer for {}", target_user_id);
        let offer = wasm_bindgen_futures::JsFuture::from(pc.create_offer()).await?;
        let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))?
            .as_string()
            .ok_or_else(|| JsValue::from_str("No SDP in offer"))?;
        
        // Set local description
        let mut offer_init = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        offer_init.sdp(&offer_sdp);
        wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&offer_init)).await?;
        
        // Send offer via WebSocket
        let msg = ClientMessage::WebrtcOffer {
            target_user_id: target_user_id.clone(),
            sdp: offer_sdp,
        };
        let msg_str = serde_json::to_string(&msg).map_err(|e| JsValue::from_str(&e.to_string()))?;
        ws.send_with_str(&msg_str)?;
        
        info!("Sent offer to {}", target_user_id);
    }
    
    Ok(pc)
}

// Handle incoming WebRTC offer
async fn handle_webrtc_offer(
    local_stream: MediaStream,
    from_user_id: String,
    ws: WebSocket,
    offer_sdp: String,
    participant_audio_levels: Signal<HashMap<String, f64>>,
) -> Result<RtcPeerConnection, JsValue> {
    info!("Handling WebRTC offer from {}", from_user_id);
    
    let pc = create_peer_connection(local_stream, from_user_id.clone(), ws.clone(), false, participant_audio_levels).await?;
    
    // Set remote description (the offer)
    let mut offer_init = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    offer_init.sdp(&offer_sdp);
    wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&offer_init)).await?;
    
    // Create answer
    let answer = wasm_bindgen_futures::JsFuture::from(pc.create_answer()).await?;
    let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("No SDP in answer"))?;
    
    // Set local description
    let mut answer_init = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    answer_init.sdp(&answer_sdp);
    wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&answer_init)).await?;
    
    // Send answer via WebSocket
    let msg = ClientMessage::WebrtcAnswer {
        target_user_id: from_user_id.clone(),
        sdp: answer_sdp,
    };
    let msg_str = serde_json::to_string(&msg).map_err(|e| JsValue::from_str(&e.to_string()))?;
    ws.send_with_str(&msg_str)?;
    
    info!("Sent answer to {}", from_user_id);
    
    Ok(pc)
}

// Handle incoming WebRTC answer
async fn handle_webrtc_answer(pc: RtcPeerConnection, answer_sdp: String) -> Result<(), JsValue> {
    info!("Setting remote description (answer)");
    
    let mut answer_init = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    answer_init.sdp(&answer_sdp);
    wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&answer_init)).await?;
    
    Ok(())
}

// Handle incoming ICE candidate
async fn handle_ice_candidate(pc: RtcPeerConnection, candidate_str: String) -> Result<(), JsValue> {
    info!("Adding ICE candidate");
    
    let mut candidate_init = RtcIceCandidateInit::new(&candidate_str);
    candidate_init.sdp_m_line_index(Some(0));
    
    wasm_bindgen_futures::JsFuture::from(pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&candidate_init))).await?;
    
    Ok(())
}

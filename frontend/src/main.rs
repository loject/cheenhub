use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, error};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    AudioContext, MessageEvent, MediaStream, UrlSearchParams, WebSocket,
    RtcPeerConnection, RtcConfiguration, RtcIceServer, RtcSessionDescriptionInit,
    RtcSdpType, RtcIceCandidateInit, RtcPeerConnectionIceEvent, RtcTrackEvent,
};
use js_sys::{Array, JsString, Reflect};

// JavaScript helper for parsing RTCStatsReport
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "parseRTCStats")]
    fn parse_rtc_stats(stats_report: &JsValue) -> js_sys::Array;
}

fn main() {
    console_error_panic_hook::set_once();

    // Initialize tracing for web console logging
    tracing_wasm::set_as_global_default_with_config(
        tracing_wasm::WASMLayerConfigBuilder::new()
            .set_max_level(tracing::Level::INFO)
            .build()
    );
    
    // Inject JavaScript helper for parsing RTCStatsReport
    let js_code = r#"
        window.parseRTCStats = function(statsReport) {
            const result = [];
            statsReport.forEach((value, key) => {
                result.push({
                    id: key,
                    type: value.type,
                    timestamp: value.timestamp,
                    data: value
                });
            });
            return result;
        };
    "#;
    let _ = js_sys::eval(js_code);
    
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
    // SFU-based WebRTC messages
    CreatePublisher,
    PublishAudio { sdp: String },
    CreateConsumer { publisher_user_id: String },
    ConsumerAnswer { consumer_id: String, sdp: String },
    PublisherIceCandidate { candidate: String },
    ConsumerIceCandidate { consumer_id: String, candidate: String },
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

// Consumer connection info for SFU topology
#[derive(Clone, Debug)]
struct ConsumerInfo {
    connection: RtcPeerConnection,
    publisher_user_id: String,
}

// Connection statistics for WebRTC peers
#[derive(Clone, Debug)]
struct ConnectionStats {
    audio_bitrate: f64,           // кбит/с
    _audio_level: f64,             // 0.0-100.0
    packet_loss: f64,             // процент (0.0-100.0)
    jitter: f64,                  // миллисекунды
    rtt: f64,                     // Round-trip time в ms
    codec_name: String,           // "opus", "pcm", etc.
    connection_state: String,     // "new", "connecting", "connected", "disconnected"
    ice_connection_state: String, // "new", "checking", "connected", etc.
    last_updated: f64,            // performance.now()
}

impl Default for ConnectionStats {
    fn default() -> Self {
        Self {
            audio_bitrate: 0.0,
            _audio_level: 0.0,
            packet_loss: 0.0,
            jitter: 0.0,
            rtt: 0.0,
            codec_name: "N/A".to_string(),
            connection_state: "new".to_string(),
            ice_connection_state: "new".to_string(),
            last_updated: 0.0,
        }
    }
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
    let mic_status = use_signal(|| MicStatus::NotRequested);
    let media_stream = use_signal(|| None::<MediaStream>);
    let audio_level = use_signal(|| 0.0);
    let mut is_muted = use_signal(|| false);
    
    // State for rooms
    let mut user_id = use_signal(|| None::<String>);
    let mut current_room = use_signal(|| None::<String>);
    let mut room_input = use_signal(|| String::from(""));
    let mut participants = use_signal(|| Vec::<Participant>::new());
    
    // SFU WebRTC state - publisher/consumer topology
    let mut publisher_connection = use_signal(|| None::<RtcPeerConnection>);
    let mut consumer_connections = use_signal(|| HashMap::<String, ConsumerInfo>::new());
    
    // Audio levels for each participant
    let mut participant_audio_levels = use_signal(|| HashMap::<String, f64>::new());
    
    // Connection statistics for each connection (keyed by user_id for consumers, "publisher" for publisher)
    let mut connection_stats = use_signal(|| HashMap::<String, ConnectionStats>::new());
    
    // Toggle for showing detailed statistics
    let mut show_detailed_stats = use_signal(|| false);
    
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
                                    info!("[SFU] Registered with user_id: {}", uid);
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
                                    info!("[SFU] Room created: {}", rid);
                                    current_room.set(Some(rid.clone()));
                                    room_input.set(rid);
                                    participants.set(vec![]);
                                }
                                ServerMessage::RoomJoined { room_id: rid, participants: parts_info } => {
                                    info!("[SFU] Joined room: {}", rid);
                                    current_room.set(Some(rid));
                                    
                                    // Convert ParticipantInfo to Participant
                                    let parts: Vec<Participant> = parts_info.iter()
                                        .map(|info| Participant {
                                            username: info.username.clone(),
                                            user_id: info.user_id.clone(),
                                        })
                                        .collect();
                                    
                                    info!("[SFU] Received {} existing participants", parts.len());
                                    participants.set(parts);

                                    // Auto-request microphone access when joining a room.
                                    request_microphone_access(
                                        mic_status,
                                        media_stream,
                                        audio_level,
                                        current_room,
                                        ws,
                                    );
                                    
                                    // If we have microphone, create publisher
                                    if media_stream.read().is_some() {
                                        info!("[SFU] Microphone available, creating publisher");
                                        let msg = ClientMessage::CreatePublisher;
                                        if let Ok(msg_str) = serde_json::to_string(&msg) {
                                            let _ = ws_for_msg.send_with_str(&msg_str);
                                        }
                                    } else {
                                        info!("[SFU] No microphone yet, publisher will be created when mic is granted");
                                    }
                                }
                                ServerMessage::UserJoined { username: uname, user_id: uid } => {
                                    info!("[SFU] User joined: {} ({})", uname, uid);
                                    
                                    // Add participant to list
                                    participants.write().push(Participant {
                                        username: uname,
                                        user_id: uid,
                                    });
                                }
                                ServerMessage::UserLeft { username: uname, user_id: uid } => {
                                    info!("[SFU] User left: {} ({})", uname, uid);
                                    participants.write().retain(|p| p.user_id != uid);
                                    
                                    // Remove consumer connection for this user
                                    let removed_consumers: Vec<String> = consumer_connections.read()
                                        .iter()
                                        .filter(|(_, info)| info.publisher_user_id == uid)
                                        .map(|(cid, _)| cid.clone())
                                        .collect();
                                    
                                    for consumer_id in removed_consumers {
                                        if let Some(info) = consumer_connections.write().remove(&consumer_id) {
                                            info!("[SFU] Closing consumer {} for user {}", consumer_id, uid);
                                            info.connection.close();
                                        }
                                    }
                                    
                                    participant_audio_levels.write().remove(&uid);
                                    connection_stats.write().remove(&uid);
                                }
                                ServerMessage::RoomLeft => {
                                    info!("[SFU] Left room");
                                    current_room.set(None);
                                    participants.set(vec![]);
                                    
                                    // Close publisher connection
                                    if let Some(pc) = publisher_connection.write().take() {
                                        info!("[SFU] Closing publisher connection");
                                        pc.close();
                                    }
                                    
                                    // Close all consumer connections
                                    for (cid, info) in consumer_connections.write().drain() {
                                        info!("[SFU] Closing consumer {}", cid);
                                        info.connection.close();
                                    }
                                    
                                    participant_audio_levels.write().clear();
                                    connection_stats.write().clear();
                                }
                                ServerMessage::Error { message: err } => {
                                    info!("[Error] Server error: {}", err);
                                }
                                ServerMessage::Pong => {
                                    // Pong received - no logging needed
                                }
                                // SFU message handlers
                                ServerMessage::PublisherCreated { sdp } => {
                                    info!("[SFU] Publisher created, received SDP offer");
                                    
                                    if let Some(stream) = media_stream.read().as_ref() {
                                        let stream_clone = stream.clone();
                                        let ws_clone = ws_for_msg.clone();
                                        
                                        spawn_local(async move {
                                            match create_publisher_connection(stream_clone, ws_clone, sdp, connection_stats).await {
                                                Ok(pc) => {
                                                    info!("[SFU] Publisher connection created successfully");
                                                    publisher_connection.set(Some(pc));
                                                }
                                                Err(e) => {
                                                    info!("[Error] Failed to create publisher connection: {:?}", e);
                                                }
                                            }
                                        });
                                    } else {
                                        info!("[Error] No media stream available for publisher");
                                    }
                                }
                                ServerMessage::AudioPublished { track_id } => {
                                    info!("[SFU] Audio published successfully, track_id: {}", track_id);
                                }
                                ServerMessage::NewPublisher { user_id: pub_uid, username: pub_name } => {
                                    info!("[SFU] New publisher available: {} ({})", pub_name, pub_uid);
                                    
                                    // Request to create consumer for this publisher
                                    let msg = ClientMessage::CreateConsumer {
                                        publisher_user_id: pub_uid,
                                    };
                                    if let Ok(msg_str) = serde_json::to_string(&msg) {
                                        let _ = ws_for_msg.send_with_str(&msg_str);
                                    }
                                }
                                ServerMessage::ConsumerCreated { consumer_id, publisher_user_id, sdp } => {
                                    info!("[SFU] Consumer created: {}, publisher: {}", consumer_id, publisher_user_id);
                                    
                                    let ws_clone = ws_for_msg.clone();
                                    let cid = consumer_id.clone();
                                    let pub_uid = publisher_user_id.clone();
                                    
                                    spawn_local(async move {
                                        match create_consumer_connection(cid.clone(), pub_uid.clone(), ws_clone, sdp, participant_audio_levels, connection_stats).await {
                                            Ok(pc) => {
                                                info!("[SFU] Consumer connection created successfully");
                                                consumer_connections.write().insert(cid, ConsumerInfo {
                                                    connection: pc,
                                                    publisher_user_id: pub_uid,
                                                });
                                            }
                                            Err(e) => {
                                                info!("[Error] Failed to create consumer connection: {:?}", e);
                                            }
                                        }
                                    });
                                }
                                ServerMessage::PublisherIceCandidate { candidate } => {
                                    info!("[SFU] Received publisher ICE candidate");
                                    
                                    if let Some(pc) = publisher_connection.read().as_ref() {
                                        let pc_clone = pc.clone();
                                        spawn_local(async move {
                                            if let Err(e) = handle_ice_candidate(pc_clone, candidate).await {
                                                info!("[Error] Failed to add publisher ICE candidate: {:?}", e);
                                            }
                                        });
                                    } else {
                                        info!("[Warning] Received publisher ICE candidate but no publisher connection");
                                    }
                                }
                                ServerMessage::ConsumerIceCandidate { consumer_id, candidate } => {
                                    info!("[SFU] Received consumer ICE candidate for {}", consumer_id);
                                    
                                    if let Some(info) = consumer_connections.read().get(&consumer_id) {
                                        let pc_clone = info.connection.clone();
                                        spawn_local(async move {
                                            if let Err(e) = handle_ice_candidate(pc_clone, candidate).await {
                                                info!("[Error] Failed to add consumer ICE candidate: {:?}", e);
                                            }
                                        });
                                    } else {
                                        info!("[Warning] Received ICE candidate for unknown consumer: {}", consumer_id);
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
                    
                    // Close publisher connection
                    if let Some(pc) = publisher_connection.write().take() {
                        pc.close();
                    }
                    
                    // Close all consumer connections
                    for (_, info) in consumer_connections.write().drain() {
                        info.connection.close();
                    }
                    
                    participant_audio_levels.write().clear();
                    connection_stats.write().clear();
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
        request_microphone_access(
            mic_status,
            media_stream,
            audio_level,
            current_room,
            ws,
        );
    };
    
    // Handler for muting/unmuting microphone
    let toggle_mute = move |_: Event<MouseData>| {
        if let Some(stream) = media_stream.read().as_ref() {
            let tracks = stream.get_audio_tracks();
            let new_muted_state = !is_muted.read().clone();
            
            for i in 0..tracks.length() {
                if let Some(track) = tracks.get(i).dyn_into::<web_sys::MediaStreamTrack>().ok() {
                    track.set_enabled(!new_muted_state);
                }
            }
            
            is_muted.set(new_muted_state);
            info!("[Audio] Microphone {}", if new_muted_state { "muted" } else { "unmuted" });
        }
    };

    rsx! {
        style { {include_str!("../style.css")} }
        
        div { class: "container",
            h1 { "Voice Messenger PoC (SFU)" }
            
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
            
            // Mute/Unmute button
            if *mic_status.read() == MicStatus::Allowed {
                button {
                    class: if *is_muted.read() { "mute-btn muted" } else { "mute-btn" },
                    onclick: toggle_mute,
                    if *is_muted.read() { "🔇 Unmute Microphone" } else { "🔊 Mute Microphone" }
                }
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
                                
                                // Toggle for detailed statistics
                                div { class: "stats-toggle",
                                    label {
                                        input {
                                            r#type: "checkbox",
                                            checked: *show_detailed_stats.read(),
                                            onchange: move |evt| show_detailed_stats.set(evt.checked())
                                        }
                                        " Show detailed statistics"
                                    }
                                }
                                
                                // Participants list with conditional rendering
                                if *show_detailed_stats.read() {
                                    // Detailed stats view
                                    div { class: "participants-stats-list",
                                        for participant in participants.read().iter() {
                                            if !participant.user_id.is_empty() {
                                                {
                                                    let stats = connection_stats.read()
                                                        .get(&participant.user_id)
                                                        .cloned()
                                                        .unwrap_or_default();
                                                    let level = participant_audio_levels.read()
                                                        .get(&participant.user_id)
                                                        .copied()
                                                        .unwrap_or(0.0);
                                                    
                                                    rsx! {
                                                        div { class: "participant-stats-card",
                                                            div { class: "participant-header",
                                                                span { class: "participant-name", "{participant.username}" }
                                                                span {
                                                                    class: if stats.connection_state == "connected" {
                                                                        "connection-badge badge-connected"
                                                                    } else {
                                                                        "connection-badge badge-disconnected"
                                                                    },
                                                                    "{stats.connection_state}"
                                                                }
                                                            }
                                                            
                                                            // Audio level meter
                                                            div { class: "stats-row",
                                                                span { class: "stats-label", "Audio:" }
                                                                div { class: "audio-meter-inline",
                                                                    div {
                                                                        class: "audio-meter-fill",
                                                                        style: "width: {level}%"
                                                                    }
                                                                }
                                                            }
                                                            
                                                            // Network metrics
                                                            div { class: "stats-grid",
                                                                div { class: "stat-item",
                                                                    span { class: "stat-label", "Bitrate:" }
                                                                    span { class: "stat-value", "{stats.audio_bitrate:.1} kbps" }
                                                                }
                                                                div { class: "stat-item",
                                                                    span { class: "stat-label", "RTT:" }
                                                                    span { class: "stat-value", "{stats.rtt:.0} ms" }
                                                                }
                                                                div { class: "stat-item",
                                                                    span { class: "stat-label", "Jitter:" }
                                                                    span { class: "stat-value", "{stats.jitter:.1} ms" }
                                                                }
                                                                div { class: "stat-item",
                                                                    span { class: "stat-label", "Loss:" }
                                                                    span {
                                                                        class: if stats.packet_loss > 5.0 {
                                                                            "stat-value stat-warning"
                                                                        } else {
                                                                            "stat-value"
                                                                        },
                                                                        "{stats.packet_loss:.1}%"
                                                                    }
                                                                }
                                                                div { class: "stat-item",
                                                                    span { class: "stat-label", "Codec:" }
                                                                    span { class: "stat-value", "{stats.codec_name}" }
                                                                }
                                                                div { class: "stat-item",
                                                                    span { class: "stat-label", "ICE:" }
                                                                    span { class: "stat-value-small", "{stats.ice_connection_state}" }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // Compact view
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
                    li { "🎙️ SFU topology: 1 publisher + N consumers (better scalability)" }
                    li { "Check browser console for detailed logs" }
                }
            }
        }
    }
}

fn request_microphone_access(
    mut mic_status: Signal<MicStatus>,
    mut media_stream: Signal<Option<MediaStream>>,
    audio_level: Signal<f64>,
    current_room: Signal<Option<String>>,
    ws: Signal<Option<WebSocket>>,
) {
    if *mic_status.read() != MicStatus::NotRequested {
        return;
    }

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

        // Create constraints for audio only with optimizations for low latency
        let constraints = web_sys::MediaStreamConstraints::new();

        // Create advanced audio constraints object
        let audio_constraints_obj = js_sys::Object::new();

        // Enable audio processing features
        if let Err(e) = js_sys::Reflect::set(&audio_constraints_obj, &JsValue::from("echoCancellation"), &JsValue::from(true)) {
            info!("[Warning] Failed to set echoCancellation: {:?}", e);
        }
        if let Err(e) = js_sys::Reflect::set(&audio_constraints_obj, &JsValue::from("noiseSuppression"), &JsValue::from(true)) {
            info!("[Warning] Failed to set noiseSuppression: {:?}", e);
        }
        if let Err(e) = js_sys::Reflect::set(&audio_constraints_obj, &JsValue::from("autoGainControl"), &JsValue::from(true)) {
            info!("[Warning] Failed to set autoGainControl: {:?}", e);
        }
        // Set latency to 10ms for low-latency audio capture
        if let Err(e) = js_sys::Reflect::set(&audio_constraints_obj, &JsValue::from("latency"), &JsValue::from(0.01)) {
            info!("[Warning] Failed to set latency: {:?}", e);
        }

        constraints.set_audio(&audio_constraints_obj.into());
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

                // If we're already in a room, create publisher
                if current_room.read().is_some() {
                    info!("[SFU] Already in room, creating publisher");

                    if let Some(websocket) = ws.read().as_ref() {
                        let msg = ClientMessage::CreatePublisher;
                        if let Ok(msg_str) = serde_json::to_string(&msg) {
                            let _ = websocket.send_with_str(&msg_str);
                        }
                    }
                }
            }
            Err(e) => {
                info!("Microphone access denied: {:?}", e);
                mic_status.set(MicStatus::Denied);
            }
        }
    });
}

// Function to start audio analysis and update audio level
fn start_audio_analysis(stream: MediaStream, mut audio_level: Signal<f64>) {
    spawn_local(async move {
        // Create optimized AudioContext for low latency
        let audio_options = web_sys::AudioContextOptions::new();
        audio_options.set_latency_hint(&JsValue::from_str("interactive"));
        audio_options.set_sample_rate(48000.0);
        
        let audio_context = match AudioContext::new_with_context_options(&audio_options) {
            Ok(ctx) => ctx,
            Err(_) => {
                // Fallback to default AudioContext if options not supported
                info!("[Audio] AudioContextOptions not supported, using default");
                match AudioContext::new() {
                    Ok(ctx) => ctx,
                    Err(e) => {
                        info!("[Error] Failed to create AudioContext: {:?}", e);
                        return;
                    }
                }
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
        // Reduce FFT size for lower latency (512 samples @ 48kHz = ~10ms)
        analyser.set_fft_size(512);
        // Add smoothing for more stable audio level readings
        analyser.set_smoothing_time_constant(0.3);
        
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
        // Create optimized AudioContext for low latency
        let audio_options = web_sys::AudioContextOptions::new();
        audio_options.set_latency_hint(&JsValue::from_str("interactive"));
        audio_options.set_sample_rate(48000.0);
        
        let audio_context = match AudioContext::new_with_context_options(&audio_options) {
            Ok(ctx) => ctx,
            Err(_) => {
                // Fallback to default AudioContext if options not supported
                info!("[Audio] AudioContextOptions not supported for {}, using default", user_id);
                match AudioContext::new() {
                    Ok(ctx) => ctx,
                    Err(e) => {
                        info!("[Error] Failed to create AudioContext for remote stream {}: {:?}", user_id, e);
                        return;
                    }
                }
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
        // Reduce FFT size for lower latency (512 samples @ 48kHz = ~10ms)
        analyser.set_fft_size(512);
        // Add smoothing for more stable audio level readings
        analyser.set_smoothing_time_constant(0.3);
        
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

// Thread-local storage for tracking previous stats values
use std::cell::RefCell;

thread_local! {
    static PREV_STATS: RefCell<HashMap<String, (f64, f64, f64)>> = RefCell::new(HashMap::new());
}

// Collect WebRTC statistics for a peer connection
async fn collect_peer_stats(
    pc: RtcPeerConnection,
    user_id: String,
    mut connection_stats: Signal<HashMap<String, ConnectionStats>>,
) -> Result<(), JsValue> {
    use wasm_bindgen_futures::JsFuture;
    
    info!("[Stats] Starting stats collection for {}", user_id);
    
    loop {
        // Wait 1 second between stat collections
        gloo_timers::future::TimeoutFuture::new(1000).await;
        
        // Get RTCStatsReport
        let stats_promise = pc.get_stats();
        let stats_result = match JsFuture::from(stats_promise).await {
            Ok(result) => result,
            Err(e) => {
                info!("[Stats] Failed to get stats for {}: {:?}, connection may be closed", user_id, e);
                break;
            }
        };
        
        let mut current_stats = ConnectionStats::default();
        
        // Use JavaScript helper to parse RTCStatsReport
        let stats_array = parse_rtc_stats(&stats_result);
        
        // Get previous stats values from thread-local storage
        let (mut prev_bytes_sent, mut prev_bytes_received, mut prev_timestamp) =
            PREV_STATS.with(|map| {
                map.borrow().get(&user_id).copied().unwrap_or((0.0, 0.0, 0.0))
            });
        
        // Iterate through all stat entries
        for i in 0..stats_array.length() {
            if let Some(stat_obj) = stats_array.get(i).dyn_into::<js_sys::Object>().ok() {
                // Get stat type
                if let Ok(type_val) = Reflect::get(&stat_obj, &JsValue::from_str("type")) {
                    if let Some(stat_type) = type_val.as_string() {
                        // Get the data object
                        if let Ok(data) = Reflect::get(&stat_obj, &JsValue::from_str("data")) {
                            match stat_type.as_str() {
                                "inbound-rtp" => {
                                    // Check if this is audio
                                    if let Ok(kind) = Reflect::get(&data, &JsValue::from_str("kind")) {
                                        let kind_str = kind.as_string().unwrap_or_default();
                                        if kind_str == "audio" {
                                            // Packets received and lost for packet loss percentage
                                            let packets_received = Reflect::get(&data, &JsValue::from_str("packetsReceived"))
                                                .ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
                                            
                                            let packets_lost = Reflect::get(&data, &JsValue::from_str("packetsLost"))
                                                .ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
                                            
                                            if packets_received > 0.0 {
                                                current_stats.packet_loss = (packets_lost / (packets_received + packets_lost)) * 100.0;
                                            }
                                            
                                            // Jitter (in seconds, convert to ms)
                                            if let Some(jitter) = Reflect::get(&data, &JsValue::from_str("jitter"))
                                                .ok().and_then(|v| v.as_f64()) {
                                                current_stats.jitter = jitter * 1000.0;
                                            }
                                            
                                            // Bytes received for bitrate calculation
                                            if let Some(bytes_received) = Reflect::get(&data, &JsValue::from_str("bytesReceived"))
                                                .ok().and_then(|v| v.as_f64()) {
                                                if let Some(timestamp) = Reflect::get(&data, &JsValue::from_str("timestamp"))
                                                    .ok().and_then(|v| v.as_f64()) {
                                                    
                                                    if prev_bytes_received > 0.0 && prev_timestamp > 0.0 {
                                                        let bytes_delta = bytes_received - prev_bytes_received;
                                                        let time_delta = (timestamp - prev_timestamp) / 1000.0; // ms to seconds
                                                        
                                                        if time_delta > 0.0 {
                                                            current_stats.audio_bitrate = (bytes_delta * 8.0) / (time_delta * 1000.0); // kbps
                                                        }
                                                    }
                                                    
                                                    prev_bytes_received = bytes_received;
                                                    prev_timestamp = timestamp;
                                                }
                                            }
                                        }
                                    }
                                }
                                "outbound-rtp" => {
                                    // Check if this is audio
                                    if let Ok(kind) = Reflect::get(&data, &JsValue::from_str("kind")) {
                                        let kind_str = kind.as_string().unwrap_or_default();
                                        if kind_str == "audio" {
                                            // Bytes sent for bitrate calculation
                                            if let Some(bytes_sent) = Reflect::get(&data, &JsValue::from_str("bytesSent"))
                                                .ok().and_then(|v| v.as_f64()) {
                                                if let Some(timestamp) = Reflect::get(&data, &JsValue::from_str("timestamp"))
                                                    .ok().and_then(|v| v.as_f64()) {
                                                    
                                                    if prev_bytes_sent > 0.0 && prev_timestamp > 0.0 {
                                                        let bytes_delta = bytes_sent - prev_bytes_sent;
                                                        let time_delta = (timestamp - prev_timestamp) / 1000.0;
                                                        
                                                        if time_delta > 0.0 {
                                                            current_stats.audio_bitrate = (bytes_delta * 8.0) / (time_delta * 1000.0);
                                                        }
                                                    }
                                                    
                                                    prev_bytes_sent = bytes_sent;
                                                    if prev_timestamp == 0.0 {
                                                        prev_timestamp = timestamp;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                "candidate-pair" => {
                                    // Check if this candidate pair is selected/active
                                    if let Ok(state) = Reflect::get(&data, &JsValue::from_str("state")) {
                                        let state_str = state.as_string().unwrap_or_default();
                                        if state_str == "succeeded" {
                                            // RTT (round-trip time)
                                            if let Some(rtt) = Reflect::get(&data, &JsValue::from_str("currentRoundTripTime"))
                                                .ok().and_then(|v| v.as_f64()) {
                                                current_stats.rtt = rtt * 1000.0; // convert to ms
                                            }
                                        }
                                    }
                                }
                                "codec" => {
                                    // Codec information
                                    if let Some(mime_str) = Reflect::get(&data, &JsValue::from_str("mimeType"))
                                        .ok().and_then(|v| v.as_string()) {
                                        if mime_str.contains("audio") {
                                            // Extract codec name (e.g., "audio/opus" -> "opus")
                                            current_stats.codec_name = mime_str
                                                .split('/')
                                                .nth(1)
                                                .unwrap_or("unknown")
                                                .to_string();
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        
        // Save stats to thread-local storage
        PREV_STATS.with(|map| {
            map.borrow_mut().insert(user_id.clone(), (prev_bytes_sent, prev_bytes_received, prev_timestamp));
        });
        
        // Get connection states
        current_stats.connection_state = match pc.connection_state() {
            web_sys::RtcPeerConnectionState::New => "new",
            web_sys::RtcPeerConnectionState::Connecting => "connecting",
            web_sys::RtcPeerConnectionState::Connected => "connected",
            web_sys::RtcPeerConnectionState::Disconnected => "disconnected",
            web_sys::RtcPeerConnectionState::Failed => "failed",
            web_sys::RtcPeerConnectionState::Closed => "closed",
            _ => "unknown",
        }.to_string();
        
        current_stats.ice_connection_state = match pc.ice_connection_state() {
            web_sys::RtcIceConnectionState::New => "new",
            web_sys::RtcIceConnectionState::Checking => "checking",
            web_sys::RtcIceConnectionState::Connected => "connected",
            web_sys::RtcIceConnectionState::Completed => "completed",
            web_sys::RtcIceConnectionState::Failed => "failed",
            web_sys::RtcIceConnectionState::Disconnected => "disconnected",
            web_sys::RtcIceConnectionState::Closed => "closed",
            _ => "unknown",
        }.to_string();
        
        // Set timestamp
        if let Some(window) = web_sys::window() {
            if let Some(performance) = window.performance() {
                current_stats.last_updated = performance.now();
            }
        }
        
        // Update the stats map
        connection_stats.write().insert(user_id.clone(), current_stats);
        
        // Break if connection is closed
        if pc.connection_state() == web_sys::RtcPeerConnectionState::Closed {
            info!("[Stats] Connection closed for {}, stopping stats collection", user_id);
            break;
        }
    }
    
    Ok(())
}

// Create RTCPeerConnection with ICE servers optimized for low latency
fn create_rtc_peer_connection() -> Result<RtcPeerConnection, JsValue> {
    let config = RtcConfiguration::new();
    
    // Add STUN servers (using Google's public STUN servers)
    let ice_servers = Array::new();
    let stun_server = RtcIceServer::new();
    stun_server.set_urls(&JsValue::from_str("stun:stun.l.google.com:19302"));
    ice_servers.push(&stun_server);
    
    let stun_server2 = RtcIceServer::new();
    stun_server2.set_urls(&JsValue::from_str("stun:stun1.l.google.com:19302"));
    ice_servers.push(&stun_server2);
    
    config.set_ice_servers(&ice_servers);
    
    // Use balanced bundle policy for compatibility with webrtc-rs backend
    // webrtc-rs doesn't generate BUNDLE group in SDP, so MaxBundle would fail
    config.set_bundle_policy(web_sys::RtcBundlePolicy::Balanced);
    
    info!("[WebRTC] Creating peer connection with low-latency configuration");
    
    RtcPeerConnection::new_with_configuration(&config)
}

// Create publisher connection (sends local audio to SFU)
async fn create_publisher_connection(
    local_stream: MediaStream,
    ws: WebSocket,
    offer_sdp: String,
    connection_stats: Signal<HashMap<String, ConnectionStats>>,
) -> Result<RtcPeerConnection, JsValue> {
    info!("[SFU] Creating publisher connection");
    
    let pc = create_rtc_peer_connection()?;
    
    // Add local audio track
    let tracks = local_stream.get_tracks();
    for i in 0..tracks.length() {
        if let Some(track) = tracks.get(i).dyn_into::<web_sys::MediaStreamTrack>().ok() {
            info!("[SFU] Adding local track to publisher");
            let _ = pc.add_track(&track, &local_stream, &Array::new());
        }
    }
    
    // Set up ICE candidate handler
    let ws_clone = ws.clone();
    let onicecandidate = Closure::wrap(Box::new(move |ev: RtcPeerConnectionIceEvent| {
        if let Some(candidate) = ev.candidate() {
            info!("[SFU] Publisher ICE candidate generated");
            let candidate_json = candidate.to_json();
            
            if let Ok(candidate_str) = Reflect::get(&candidate_json, &JsValue::from_str("candidate")) {
                if let Some(cand_str) = candidate_str.as_string() {
                    let msg = ClientMessage::PublisherIceCandidate {
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
    
    // Set remote description (offer from server)
    let offer_init = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    offer_init.set_sdp(&offer_sdp);
    info!("[SFU Publisher] Setting remote description (offer)");
    wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&offer_init)).await?;
    
    // STEP 1: Log state after setRemoteDescription
    info!("[SFU Publisher] Remote description set successfully");
    info!("[SFU Publisher] ICE connection state: {:?}", pc.ice_connection_state());
    info!("[SFU Publisher] Connection state: {:?}", pc.connection_state());
    
    // Create answer
    info!("[SFU Publisher] Creating answer");
    let answer = wasm_bindgen_futures::JsFuture::from(pc.create_answer()).await?;
    let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("No SDP in answer"))?;
    
    // STEP 2: Log answer SDP BEFORE setLocalDescription
    info!("[SFU Publisher] Answer created");
    let answer_sdp_before = answer_sdp.clone();
    info!("[SFU Publisher] Answer SDP length BEFORE setLocalDescription: {}", answer_sdp_before.len());
    info!("[SFU Publisher] Answer contains 'a=ice-ufrag' BEFORE: {}", answer_sdp_before.contains("a=ice-ufrag"));
    info!("[SFU Publisher] Answer contains 'a=ice-pwd' BEFORE: {}", answer_sdp_before.contains("a=ice-pwd"));
    // Log first 500 characters of SDP for inspection
    let preview_len = answer_sdp_before.len().min(500);
    info!("[SFU Publisher] Answer SDP preview: {}", &answer_sdp_before[..preview_len]);
    
    // Set local description
    let answer_init = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    answer_init.set_sdp(&answer_sdp);
    info!("[SFU Publisher] Setting local description (answer)");
    wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&answer_init)).await?;
    
    // STEP 3: Log state after setLocalDescription
    info!("[SFU Publisher] Local description set");
    info!("[SFU Publisher] ICE connection state AFTER setLocal: {:?}", pc.ice_connection_state());
    
    // CRITICAL FIX: Wait for browser to populate ICE credentials in SDP
    // After setLocalDescription, the browser needs time to add ice-ufrag and ice-pwd
    info!("[SFU Publisher] Waiting 100ms for ICE credentials to be added to SDP...");
    gloo_timers::future::TimeoutFuture::new(100).await;
    
    // STEP 4: Log final local_description content
    let final_answer_sdp = if let Some(final_desc) = pc.local_description() {
        let final_sdp = final_desc.sdp();
        info!("[SFU Publisher] Final SDP length: {}", final_sdp.len());
        info!("[SFU Publisher] Final contains 'a=ice-ufrag': {}", final_sdp.contains("a=ice-ufrag"));
        info!("[SFU Publisher] Final contains 'a=ice-pwd': {}", final_sdp.contains("a=ice-pwd"));
        info!("[SFU Publisher] Final contains 'a=candidate': {}", final_sdp.contains("a=candidate"));
        // Log first 500 characters of final SDP
        let final_preview_len = final_sdp.len().min(500);
        info!("[SFU Publisher] Final SDP preview: {}", &final_sdp[..final_preview_len]);
        final_sdp
    } else {
        error!("[SFU Publisher] local_description() returned None!");
        answer_sdp_before
    };
    
    // Send answer to server with ICE credentials
    let msg = ClientMessage::PublishAudio { sdp: final_answer_sdp };
    let msg_str = serde_json::to_string(&msg).map_err(|e| JsValue::from_str(&e.to_string()))?;
    ws.send_with_str(&msg_str)?;
    
    info!("[SFU] Publisher answer sent with ICE credentials");
    
    // Start collecting stats for publisher
    {
        let pc_clone = pc.clone();
        spawn_local(async move {
            if let Err(e) = collect_peer_stats(pc_clone, "publisher".to_string(), connection_stats).await {
                info!("[Stats] Publisher stats collection ended: {:?}", e);
            }
        });
    }
    
    Ok(pc)
}

// Create consumer connection (receives audio from SFU for a specific publisher)
async fn create_consumer_connection(
    consumer_id: String,
    publisher_user_id: String,
    ws: WebSocket,
    offer_sdp: String,
    participant_audio_levels: Signal<HashMap<String, f64>>,
    connection_stats: Signal<HashMap<String, ConnectionStats>>,
) -> Result<RtcPeerConnection, JsValue> {
    info!("[SFU] Creating consumer connection for publisher {}", publisher_user_id);
    
    let pc = create_rtc_peer_connection()?;
    
    // Set up ICE candidate handler
    let ws_clone = ws.clone();
    let cid_clone = consumer_id.clone();
    let onicecandidate = Closure::wrap(Box::new(move |ev: RtcPeerConnectionIceEvent| {
        if let Some(candidate) = ev.candidate() {
            info!("[SFU] Consumer {} ICE candidate generated", cid_clone);
            let candidate_json = candidate.to_json();
            
            if let Ok(candidate_str) = Reflect::get(&candidate_json, &JsValue::from_str("candidate")) {
                if let Some(cand_str) = candidate_str.as_string() {
                    let msg = ClientMessage::ConsumerIceCandidate {
                        consumer_id: cid_clone.clone(),
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
    
    // Set up ontrack handler to receive audio
    let pub_uid = publisher_user_id.clone();
    let ontrack = Closure::wrap(Box::new(move |ev: RtcTrackEvent| {
        info!("[SFU] Received track from publisher {}", pub_uid);
        
        let streams = ev.streams();
        if streams.length() > 0 {
            if let Some(remote_stream) = streams.get(0).dyn_into::<MediaStream>().ok() {
                // Play the remote audio stream
                match web_sys::HtmlAudioElement::new() {
                    Ok(audio) => {
                        audio.set_src_object(Some(&remote_stream));
                        audio.set_autoplay(true);
                        match audio.play() {
                            Ok(_) => {
                                info!("[Audio] Started playing audio from {}", pub_uid);
                            }
                            Err(e) => {
                                info!("[Error] Failed to play audio from {}: {:?}", pub_uid, e);
                            }
                        }
                        
                        // Start audio analysis
                        start_remote_audio_analysis(remote_stream, pub_uid.clone(), participant_audio_levels);
                    }
                    Err(e) => {
                        info!("[Error] Failed to create audio element: {:?}", e);
                    }
                }
            }
        }
    }) as Box<dyn FnMut(RtcTrackEvent)>);
    
    pc.set_ontrack(Some(ontrack.as_ref().unchecked_ref()));
    ontrack.forget();
    
    // Set remote description (offer from server)
    let offer_init = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    offer_init.set_sdp(&offer_sdp);
    info!("[WebRTC] Setting remote description (offer) for consumer {}", consumer_id);
    wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&offer_init)).await?;
    
    // Create answer
    info!("[WebRTC] Creating answer for consumer {}", consumer_id);
    let answer = wasm_bindgen_futures::JsFuture::from(pc.create_answer()).await?;
    let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("No SDP in answer"))?;
    
    // Debug: Check SDP before setting local description
    info!("[WebRTC] Consumer answer SDP length: {} bytes", answer_sdp.len());
    info!("[WebRTC] Consumer answer contains ice-ufrag: {}", answer_sdp.contains("ice-ufrag"));
    info!("[WebRTC] Consumer answer contains ice-pwd: {}", answer_sdp.contains("ice-pwd"));
    
    // Set local description
    let answer_init = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    answer_init.set_sdp(&answer_sdp);
    info!("[WebRTC] Setting local description (answer) for consumer {}", consumer_id);
    wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&answer_init)).await?;
    
    // CRITICAL FIX: Wait for browser to populate ICE credentials in SDP
    // After setLocalDescription, the browser needs time to add ice-ufrag and ice-pwd
    info!("[WebRTC] Waiting for ICE credentials to be added to consumer SDP...");
    gloo_timers::future::TimeoutFuture::new(100).await;
    
    // Get the local description again after ICE gathering has started
    // The browser will have updated it with ICE credentials
    let final_answer_sdp = if let Some(local_desc) = pc.local_description() {
        let final_sdp = local_desc.sdp();
        info!("[WebRTC] Final consumer answer SDP length: {} bytes", final_sdp.len());
        info!("[WebRTC] Final consumer answer contains ice-ufrag: {}", final_sdp.contains("ice-ufrag"));
        info!("[WebRTC] Final consumer answer contains ice-pwd: {}", final_sdp.contains("ice-pwd"));
        final_sdp
    } else {
        info!("[Warning] No local description available for consumer, using original answer");
        answer_sdp
    };
    
    // Send answer to server with ICE credentials
    let msg = ClientMessage::ConsumerAnswer {
        consumer_id: consumer_id.clone(),
        sdp: final_answer_sdp,
    };
    let msg_str = serde_json::to_string(&msg).map_err(|e| JsValue::from_str(&e.to_string()))?;
    ws.send_with_str(&msg_str)?;
    
    info!("[SFU] Consumer answer sent for {} with ICE credentials", consumer_id);
    
    // Start collecting stats for this consumer
    {
        let pc_clone = pc.clone();
        let pub_uid_clone = publisher_user_id.clone();
        spawn_local(async move {
            if let Err(e) = collect_peer_stats(pc_clone, pub_uid_clone.clone(), connection_stats).await {
                info!("[Stats] Consumer stats for {} ended: {:?}", pub_uid_clone, e);
            }
        });
    }
    
    Ok(pc)
}

// Handle incoming ICE candidate
async fn handle_ice_candidate(pc: RtcPeerConnection, candidate_str: String) -> Result<(), JsValue> {
    info!("[ICE] Adding ICE candidate");
    
    let candidate_init = RtcIceCandidateInit::new(&candidate_str);
    candidate_init.set_sdp_m_line_index(Some(0));
    
    wasm_bindgen_futures::JsFuture::from(pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&candidate_init))).await?;
    
    Ok(())
}

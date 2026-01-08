use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::info;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket, MediaStream, AudioContext, UrlSearchParams};
use js_sys::JsString;

fn main() {
    // Initialize tracing for web console logging
    tracing_wasm::set_as_global_default();
    
    dioxus::launch(App);
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
    let mut participants = use_signal(|| Vec::<String>::new());
    
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
                        info!("Received message: {}", message);
                        
                        // Parse server message
                        if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&message) {
                            match server_msg {
                                ServerMessage::Registered { user_id: uid } => {
                                    info!("Registered with user_id: {}", uid);
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
                                    info!("Room created: {}", rid);
                                    current_room.set(Some(rid.clone()));
                                    room_input.set(rid);
                                    participants.set(vec![]);
                                }
                                ServerMessage::RoomJoined { room_id: rid, participants: parts } => {
                                    info!("Joined room: {}", rid);
                                    current_room.set(Some(rid));
                                    participants.set(parts);
                                }
                                ServerMessage::UserJoined { username: uname } => {
                                    info!("User joined: {}", uname);
                                    participants.write().push(uname);
                                }
                                ServerMessage::UserLeft { username: uname } => {
                                    info!("User left: {}", uname);
                                    participants.write().retain(|p| p != &uname);
                                }
                                ServerMessage::RoomLeft => {
                                    info!("Left room");
                                    current_room.set(None);
                                    participants.set(vec![]);
                                }
                                ServerMessage::Error { message: err } => {
                                    info!("Server error: {}", err);
                                }
                                ServerMessage::Pong => {
                                    info!("Received pong");
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
        
        spawn(async move {
            let window = web_sys::window().expect("no global window");
            let navigator = window.navigator();
            let media_devices = navigator.media_devices().expect("no media devices");
            
            // Create constraints for audio only
            let constraints = web_sys::MediaStreamConstraints::new();
            constraints.set_audio(&JsValue::from(true));
            constraints.set_video(&JsValue::from(false));
            
            match wasm_bindgen_futures::JsFuture::from(
                media_devices.get_user_media_with_constraints(&constraints).expect("failed to call getUserMedia")
            ).await {
                Ok(stream) => {
                    info!("Microphone access granted");
                    let stream: MediaStream = stream.dyn_into().expect("not a MediaStream");
                    
                    // Start audio analysis
                    start_audio_analysis(stream.clone(), audio_level);
                    
                    media_stream.set(Some(stream));
                    mic_status.set(MicStatus::Allowed);
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
                                        li { "{participant}" }
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
                    li { "Check browser console for detailed logs" }
                }
            }
        }
    }
}

// Function to start audio analysis and update audio level
fn start_audio_analysis(stream: MediaStream, mut audio_level: Signal<f64>) {
    spawn(async move {
        let audio_context = AudioContext::new().expect("failed to create AudioContext");
        let source = audio_context.create_media_stream_source(&stream)
            .expect("failed to create media stream source");
        
        let analyser = audio_context.create_analyser()
            .expect("failed to create analyser");
        analyser.set_fft_size(2048);
        
        source.connect_with_audio_node(&analyser)
            .expect("failed to connect source to analyser");
        
        let buffer_length = analyser.frequency_bin_count();
        
        // Use setInterval instead of requestAnimationFrame for simplicity
        let window = web_sys::window().expect("no window");
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
        window.set_interval_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            50
        ).expect("failed to set interval");
        
        // Keep closure alive
        closure.forget();
    });
}

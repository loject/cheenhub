use dioxus::prelude::*;
use tracing::info;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket, MediaStream, AudioContext};
use js_sys::JsString;

fn main() {
    // Initialize tracing for web console logging
    tracing_wasm::set_as_global_default();
    
    dioxus::launch(App);
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

    // Handler for connecting to the server
    let connect = move |_| {
        let username_val = username.read().clone();
        
        if username_val.is_empty() {
            info!("Username is empty, not connecting");
            return;
        }

        info!("Attempting to connect to WebSocket server...");
        
        // Create WebSocket connection
        match WebSocket::new("ws://localhost:8080/ws") {
            Ok(websocket) => {
                info!("WebSocket created successfully");
                
                // Clone for closures
                let ws_clone = websocket.clone();
                
                // Set up onopen handler
                let onopen = Closure::wrap(Box::new(move |_| {
                    info!("WebSocket connection opened");
                    status.set("Connected".to_string());
                    
                    // Send ping message on connection
                    if let Err(e) = ws_clone.send_with_str("ping") {
                        info!("Failed to send ping: {:?}", e);
                    } else {
                        info!("Sent ping message");
                    }
                }) as Box<dyn FnMut(JsValue)>);
                
                websocket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
                onopen.forget();
                
                // Set up onmessage handler
                let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                    if let Ok(txt) = e.data().dyn_into::<JsString>() {
                        let message: String = txt.into();
                        info!("Received message: {}", message);
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
                    oninput: move |evt| username.set(evt.value().clone())
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
            
            div { class: "info",
                p { "Instructions:" }
                ul {
                    li { "Enter your username" }
                    li { "Click 'Connect to Server' to establish WebSocket connection" }
                    li { "Click 'Request Microphone Access' to allow microphone usage" }
                    li { "You will see audio level indicator when microphone is active" }
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

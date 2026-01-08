use dioxus::prelude::*;
use tracing::info;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};
use js_sys::JsString;

fn main() {
    // Initialize tracing for web console logging
    tracing_wasm::set_as_global_default();
    
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // State for username input
    let mut username = use_signal(|| String::from(""));
    
    // State for connection status
    let mut status = use_signal(|| "Disconnected".to_string());
    
    // State to hold the WebSocket connection
    let mut ws = use_signal(|| None::<WebSocket>);

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

    rsx! {
        style { {include_str!("../style.css")} }
        
        div { class: "container",
            h1 { "Voice Messenger PoC" }
            
            div { class: "status-bar",
                span { "Status: " }
                span { 
                    class: if status.read().starts_with("Connected") { "status-connected" } else { "status-disconnected" },
                    "{status}"
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
            
            div { class: "info",
                p { "Instructions:" }
                ul {
                    li { "Enter your username" }
                    li { "Click 'Connect to Server' to establish WebSocket connection" }
                    li { "The app will automatically send a ping message when connected" }
                    li { "Check browser console for detailed logs" }
                }
            }
        }
    }
}

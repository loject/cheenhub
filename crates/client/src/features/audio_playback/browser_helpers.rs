//! Browser audio playback helper APIs.

use dioxus::prelude::{info, warn};
use js_sys::{Function, Object, Promise, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{AudioBufferSourceNode, AudioContext};

pub(super) fn apply_output_device_to_context(context: AudioContext, device_id: String) {
    spawn_local(async move {
        match set_context_sink_id(&context, &device_id) {
            Ok(promise) => {
                if let Err(error) = JsFuture::from(promise).await {
                    warn!(
                        error = %js_error_message(error),
                        "failed to apply audio output device to playback context"
                    );
                } else {
                    info!("applied audio output device to playback context");
                }
            }
            Err(error) => {
                warn!(
                    error = %js_error_message(error),
                    "browser does not support applying audio output device to playback context"
                );
            }
        }
    });
}

pub(super) fn set_property(object: &Object, name: &str, value: &JsValue) {
    let _ = Reflect::set(object, &JsValue::from_str(name), value);
}

pub(super) fn stop_audio_source(source: &AudioBufferSourceNode) -> Result<(), JsValue> {
    let stop = Reflect::get(source.as_ref(), &JsValue::from_str("stop"))?.dyn_into::<Function>()?;
    stop.call1(source.as_ref(), &JsValue::from_f64(0.0))
        .map(|_| ())
}

pub(super) fn js_error_message(error: JsValue) -> String {
    error
        .dyn_ref::<js_sys::Error>()
        .map(js_sys::Error::message)
        .and_then(|message| message.as_string())
        .filter(|message| !message.is_empty())
        .or_else(|| error.as_string())
        .unwrap_or_else(|| "unknown browser audio error".to_owned())
}

fn set_context_sink_id(context: &AudioContext, device_id: &str) -> Result<Promise, JsValue> {
    let set_sink_id =
        Reflect::get(context.as_ref(), &JsValue::from_str("setSinkId"))?.dyn_into::<Function>()?;
    set_sink_id
        .call1(context.as_ref(), &JsValue::from_str(device_id))?
        .dyn_into::<Promise>()
}

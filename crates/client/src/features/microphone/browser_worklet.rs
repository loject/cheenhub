//! Browser microphone AudioWorklet graph helpers.

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AudioContext, AudioContextOptions, AudioWorkletNode, AudioWorkletNodeOptions, BaseAudioContext,
    GainNode, MediaStreamAudioSourceNode, Worklet,
};

use super::backend::MicrophoneError;
use super::browser_errors::js_error_message;

const MICROPHONE_WORKLET_URL: &str = "/audio/microphone-worklet.js?v=1";
const WORKLET_PROCESSOR_NAME: &str = "cheenhub-microphone-capture";
const WORKLET_CHUNK_MS: u32 = 10;

pub(super) fn worklet_chunk_ms() -> u32 {
    WORKLET_CHUNK_MS
}

pub(super) fn create_audio_context(sample_rate_hz: u32) -> Result<AudioContext, MicrophoneError> {
    let options = AudioContextOptions::new();
    options.set_latency_hint(&JsValue::from_str("interactive"));
    options.set_sample_rate(sample_rate_hz as f32);
    AudioContext::new_with_context_options(&options).map_err(microphone_error)
}

pub(super) async fn load_worklet_module(context: &AudioContext) -> Result<(), MicrophoneError> {
    let worklet = context.audio_worklet().map_err(microphone_error)?;
    let worklet = worklet.unchecked_ref::<Worklet>();
    let promise = worklet
        .add_module(MICROPHONE_WORKLET_URL)
        .map_err(microphone_error)?;
    JsFuture::from(promise).await.map_err(microphone_error)?;
    Ok(())
}

pub(super) fn create_worklet_node(
    context: &AudioContext,
    sample_rate_hz: u32,
) -> Result<AudioWorkletNode, MicrophoneError> {
    let processor_options = Object::new();
    set_property(
        &processor_options,
        "frameCount",
        &JsValue::from_f64(f64::from(worklet_frame_count(sample_rate_hz))),
    )?;

    let options = AudioWorkletNodeOptions::new();
    options.set_number_of_inputs(1);
    options.set_number_of_outputs(1);
    options.set_output_channel_count(&output_channel_count().into());
    options.set_processor_options(Some(&processor_options));

    let context = context.unchecked_ref::<BaseAudioContext>();
    AudioWorkletNode::new_with_options(context, WORKLET_PROCESSOR_NAME, &options)
        .map_err(microphone_error)
}

pub(super) fn connect_capture_graph(
    source: &MediaStreamAudioSourceNode,
    worklet: &AudioWorkletNode,
    silent_gain: &GainNode,
    context: &AudioContext,
) -> Result<(), MicrophoneError> {
    source
        .connect_with_audio_node(worklet.as_ref())
        .map_err(microphone_error)?;
    worklet
        .connect_with_audio_node(silent_gain.as_ref())
        .map_err(microphone_error)?;
    silent_gain
        .connect_with_audio_node(context.destination().as_ref())
        .map_err(microphone_error)?;
    Ok(())
}

fn worklet_frame_count(sample_rate_hz: u32) -> u32 {
    (sample_rate_hz / (1_000 / WORKLET_CHUNK_MS)).max(128)
}

fn output_channel_count() -> Array {
    let channels = Array::new();
    channels.push(&JsValue::from_f64(1.0));
    channels
}

fn microphone_error(error: JsValue) -> MicrophoneError {
    MicrophoneError::new(js_error_message(error))
}

fn set_property(object: &Object, name: &str, value: &JsValue) -> Result<(), MicrophoneError> {
    Reflect::set(object, &JsValue::from_str(name), value)
        .map(|_| ())
        .map_err(microphone_error)
}

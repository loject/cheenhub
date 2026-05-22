//! Browser microphone gain helpers.

use js_sys::{Float32Array, Object, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use super::backend::MicrophoneError;
use super::browser_bindings::AudioData;
use super::browser_errors::js_error_message;

pub(super) struct EncodedAudioValue {
    value: JsValue,
    owned_audio: Option<AudioData>,
}

impl EncodedAudioValue {
    fn borrowed(value: &JsValue) -> Self {
        Self {
            value: value.clone(),
            owned_audio: None,
        }
    }

    fn owned(audio: AudioData) -> Self {
        Self {
            value: audio.clone().into(),
            owned_audio: Some(audio),
        }
    }

    pub(super) fn value(&self) -> &JsValue {
        &self.value
    }

    pub(super) fn close(self) {
        if let Some(audio) = self.owned_audio {
            close_audio_data(audio.as_ref());
        }
    }
}

pub(super) fn encode_value_with_gain(
    value: &JsValue,
    input_gain: f32,
) -> Result<EncodedAudioValue, String> {
    if (input_gain - 1.0).abs() < f32::EPSILON {
        return Ok(EncodedAudioValue::borrowed(value));
    }

    let audio = value.unchecked_ref::<AudioData>();
    let samples = amplified_audio_samples(audio, input_gain).map_err(|error| error.to_string())?;
    let amplified_audio = amplified_audio_data(audio, samples).map_err(js_error_message)?;
    Ok(EncodedAudioValue::owned(amplified_audio))
}

pub(super) fn apply_input_gain(samples: &mut [f32], input_gain: f32) {
    if (input_gain - 1.0).abs() < f32::EPSILON {
        return;
    }

    for sample in samples {
        *sample = (*sample * input_gain).clamp(-1.0, 1.0);
    }
}

fn amplified_audio_samples(
    audio: &AudioData,
    input_gain: f32,
) -> Result<Float32Array, MicrophoneError> {
    let frames = audio.audio_data_number_of_frames();
    let channels = audio.audio_data_number_of_channels().max(1);
    let samples = Float32Array::new_with_length(frames.saturating_mul(channels));
    for channel in 0..channels {
        let channel_samples = Float32Array::new_with_length(frames);
        audio
            .audio_data_copy_to(&channel_samples, &copy_options(channel))
            .map_err(|error| MicrophoneError::new(js_error_message(error)))?;
        for frame in 0..frames {
            let sample = (channel_samples.get_index(frame) * input_gain).clamp(-1.0, 1.0);
            samples.set_index(channel.saturating_mul(frames).saturating_add(frame), sample);
        }
    }

    Ok(samples)
}

fn amplified_audio_data(audio: &AudioData, samples: Float32Array) -> Result<AudioData, JsValue> {
    let init = Object::new();
    set_property(&init, "format", &JsValue::from_str("f32-planar"));
    set_property(
        &init,
        "sampleRate",
        &JsValue::from_f64(audio.audio_data_sample_rate()),
    );
    set_property(
        &init,
        "numberOfFrames",
        &JsValue::from_f64(f64::from(audio.audio_data_number_of_frames())),
    );
    set_property(
        &init,
        "numberOfChannels",
        &JsValue::from_f64(f64::from(audio.audio_data_number_of_channels().max(1))),
    );
    set_property(
        &init,
        "timestamp",
        &JsValue::from_f64(audio.audio_data_timestamp()),
    );
    set_property(&init, "data", samples.as_ref());
    AudioData::new(&init.into())
}

fn copy_options(plane_index: u32) -> JsValue {
    let object = Object::new();
    set_property(&object, "format", &JsValue::from_str("f32-planar"));
    set_property(
        &object,
        "planeIndex",
        &JsValue::from_f64(f64::from(plane_index)),
    );
    object.into()
}

fn close_audio_data(value: &JsValue) {
    if let Ok(close) = Reflect::get(value, &JsValue::from_str("close"))
        && let Ok(close) = close.dyn_into::<js_sys::Function>()
    {
        let _ = close.call0(value);
    }
}

fn set_property(object: &Object, name: &str, value: &JsValue) {
    let _ = Reflect::set(object, &JsValue::from_str(name), value);
}

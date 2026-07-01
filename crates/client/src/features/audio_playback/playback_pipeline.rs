//! Browser playback decoding and scheduling pipeline.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::{debug, warn};
use js_sys::{Float32Array, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use web_sys::{AudioBufferSourceNode, AudioContext, GainNode};
use web_time::Instant;

use super::AudioPlaybackInner;
use super::browser_bindings::{AudioData, AudioDecoder, EncodedAudioChunk};
use super::browser_diagnostics::{
    DecodeOutputTiming, ScheduleAudioTiming, diagnostics_enabled, elapsed_us_since,
};
use super::browser_helpers::{js_error_message, set_property};
use crate::features::audio_playback::backend::VoiceFrame;

const INITIAL_PLAYBACK_BUFFER_SECONDS: f64 = 0.03;
const CONTINUOUS_PLAYBACK_MARGIN_SECONDS: f64 = 0.02;
const PLAYBACK_SCHEDULE_WARNING_INTERVAL_SECONDS: f64 = 5.0;

pub(super) struct ScheduledAudioSource {
    pub(super) source: AudioBufferSourceNode,
    pub(super) end_time: f64,
}

pub(super) struct SenderPlayback {
    pub(super) decoder: AudioDecoder,
    pub(super) gain_node: GainNode,
    _output_closure: Closure<dyn FnMut(AudioData)>,
    _error_closure: Closure<dyn FnMut(JsValue)>,
}

pub(super) fn create_sender_playback(
    sender_user_id: String,
    context: AudioContext,
    inner: Rc<RefCell<AudioPlaybackInner>>,
    initial_gain: f64,
) -> Result<SenderPlayback, JsValue> {
    let gain_node = context.create_gain()?;
    gain_node.gain().set_value(initial_gain as f32);
    gain_node.connect_with_audio_node(&context.destination())?;

    let output_sender_id = sender_user_id.clone();
    let output_closure = Closure::wrap(Box::new(move |audio: AudioData| {
        let should_record_diagnostics = diagnostics_enabled();
        let started_at = should_record_diagnostics.then(Instant::now);
        let frames = audio.number_of_frames();
        let channels = audio.number_of_channels().max(1);
        let timestamp_us = audio.timestamp().max(0.0) as u64;
        let schedule_started_at = should_record_diagnostics.then(Instant::now);
        let schedule_result = schedule_audio_data(
            &context,
            &inner,
            &output_sender_id,
            &audio,
            should_record_diagnostics,
        );
        let schedule_elapsed_us = elapsed_us_since(&schedule_started_at);
        let close_started_at = should_record_diagnostics.then(Instant::now);
        let close_result = audio.close();
        let close_elapsed_us = elapsed_us_since(&close_started_at);
        if should_record_diagnostics && let Ok(Some(schedule_timing)) = &schedule_result {
            let mut inner = inner.borrow_mut();
            inner
                .diagnostics
                .record_schedule(&output_sender_id, schedule_timing.clone());
            inner.diagnostics.record_decode_output(
                &output_sender_id,
                DecodeOutputTiming {
                    timestamp_us,
                    frames,
                    channels,
                    total_elapsed_us: elapsed_us_since(&started_at),
                    schedule_elapsed_us,
                    close_elapsed_us,
                },
            );
        }
        if let Err(error) = schedule_result {
            warn!(
                error = %js_error_message(error),
                sender_user_id = %output_sender_id,
                "failed to schedule decoded audio"
            );
        }
        if let Err(error) = close_result {
            warn!(
                error = %js_error_message(error),
                sender_user_id = %output_sender_id,
                "failed to close decoded audio data"
            );
        }
    }) as Box<dyn FnMut(AudioData)>);
    let error_sender_id = sender_user_id.clone();
    let error_closure = Closure::wrap(Box::new(move |error: JsValue| {
        warn!(
            error = %js_error_message(error),
            sender_user_id = %error_sender_id,
            "audio decoder failed"
        );
    }) as Box<dyn FnMut(JsValue)>);
    let init = Object::new();
    Reflect::set(&init, &JsValue::from_str("output"), output_closure.as_ref())?;
    Reflect::set(&init, &JsValue::from_str("error"), error_closure.as_ref())?;
    let decoder = AudioDecoder::new(&init.into())?;
    decoder.configure(&decoder_config())?;

    Ok(SenderPlayback {
        decoder,
        gain_node,
        _output_closure: output_closure,
        _error_closure: error_closure,
    })
}

pub(super) fn encoded_audio_chunk(frame: &VoiceFrame) -> Result<EncodedAudioChunk, JsValue> {
    let data = Uint8Array::from(frame.bytes.as_slice());
    let init = Object::new();
    Reflect::set(&init, &JsValue::from_str("type"), &JsValue::from_str("key"))?;
    Reflect::set(
        &init,
        &JsValue::from_str("timestamp"),
        &JsValue::from_f64(frame.timestamp_us as f64),
    )?;
    Reflect::set(
        &init,
        &JsValue::from_str("duration"),
        &JsValue::from_f64(f64::from(frame.duration_us)),
    )?;
    Reflect::set(&init, &JsValue::from_str("data"), data.as_ref())?;
    EncodedAudioChunk::new(&init.into())
}

fn decoder_config() -> JsValue {
    let object = Object::new();
    set_property(&object, "codec", &JsValue::from_str("opus"));
    set_property(&object, "sampleRate", &JsValue::from_f64(48_000.0));
    set_property(&object, "numberOfChannels", &JsValue::from_f64(1.0));
    object.into()
}

fn schedule_audio_data(
    context: &AudioContext,
    inner: &Rc<RefCell<AudioPlaybackInner>>,
    sender_user_id: &str,
    audio: &AudioData,
    record_diagnostics: bool,
) -> Result<Option<ScheduleAudioTiming>, JsValue> {
    let started_at = record_diagnostics.then(Instant::now);
    if inner.borrow().muted {
        return Ok(None);
    }

    let frames = audio.number_of_frames();
    if frames == 0 {
        return Ok(None);
    }
    let channels = audio.number_of_channels().max(1);
    let sample_rate = audio.sample_rate().max(1.0) as f32;
    let create_buffer_started_at = record_diagnostics.then(Instant::now);
    let buffer = context.create_buffer(channels, frames, sample_rate)?;
    let create_buffer_elapsed_us = elapsed_us_since(&create_buffer_started_at);

    let mut copy_to_elapsed_us = 0_u128;
    let mut copy_channel_elapsed_us = 0_u128;
    for channel in 0..channels {
        let samples = Float32Array::new_with_length(frames);
        let copy_to_started_at = record_diagnostics.then(Instant::now);
        audio.copy_to(&samples, &copy_options(channel))?;
        copy_to_elapsed_us =
            copy_to_elapsed_us.saturating_add(elapsed_us_since(&copy_to_started_at));
        let copy_channel_started_at = record_diagnostics.then(Instant::now);
        buffer.copy_to_channel_with_f32_array(&samples, channel as i32)?;
        copy_channel_elapsed_us =
            copy_channel_elapsed_us.saturating_add(elapsed_us_since(&copy_channel_started_at));
    }

    let source_setup_started_at = record_diagnostics.then(Instant::now);
    let source = context.create_buffer_source()?;
    source.set_buffer(Some(&buffer));
    let gain_node = inner
        .borrow()
        .senders
        .get(sender_user_id)
        .map(|s| s.gain_node.clone());
    match gain_node {
        Some(gain) => source.connect_with_audio_node(&gain)?,
        None => source.connect_with_audio_node(&context.destination())?,
    };
    let source_setup_elapsed_us = elapsed_us_since(&source_setup_started_at);

    let schedule_state_started_at = record_diagnostics.then(Instant::now);
    let now = context.current_time();
    let mut inner = inner.borrow_mut();
    let previous_until = inner.scheduled_until.get(sender_user_id).copied();
    let mut underrun_ms = None;
    let mut inserted_gap_ms = None;
    let mut low_headroom_ms = None;
    let start_at = match previous_until {
        Some(previous_until) if previous_until > now => {
            let start_at = previous_until.max(now + CONTINUOUS_PLAYBACK_MARGIN_SECONDS);
            if start_at > previous_until {
                low_headroom_ms = Some((previous_until - now) * 1000.0);
                inserted_gap_ms = Some((start_at - previous_until) * 1000.0);
            }
            start_at
        }
        Some(previous_until) => {
            underrun_ms = Some((now - previous_until) * 1000.0);
            debug!(
                %sender_user_id,
                buffer_ms = INITIAL_PLAYBACK_BUFFER_SECONDS * 1000.0,
                "priming inbound voice playback buffer"
            );
            now + INITIAL_PLAYBACK_BUFFER_SECONDS
        }
        None => {
            debug!(
                %sender_user_id,
                buffer_ms = INITIAL_PLAYBACK_BUFFER_SECONDS * 1000.0,
                "priming inbound voice playback buffer"
            );
            now + INITIAL_PLAYBACK_BUFFER_SECONDS
        }
    };
    if let Some(underrun_ms) = underrun_ms {
        if should_warn_playback_schedule(&mut inner, sender_user_id, now) {
            warn!(
                %sender_user_id,
                underrun_ms,
                buffer_ms = INITIAL_PLAYBACK_BUFFER_SECONDS * 1000.0,
                "inbound voice playback underrun"
            );
        }
    } else if let (Some(low_headroom_ms), Some(inserted_gap_ms)) =
        (low_headroom_ms, inserted_gap_ms)
        && should_warn_playback_schedule(&mut inner, sender_user_id, now)
    {
        warn!(
            %sender_user_id,
            low_headroom_ms,
            inserted_gap_ms,
            margin_ms = CONTINUOUS_PLAYBACK_MARGIN_SECONDS * 1000.0,
            "inbound voice playback schedule headroom is low"
        );
    }
    let duration = f64::from(frames) / f64::from(sample_rate);
    let end_time = start_at + duration;
    inner
        .scheduled_until
        .insert(sender_user_id.to_owned(), end_time);
    let source_start_started_at = record_diagnostics.then(Instant::now);
    source.start_with_when(start_at)?;
    let source_start_elapsed_us = elapsed_us_since(&source_start_started_at);
    let sources = inner
        .scheduled_sources
        .entry(sender_user_id.to_owned())
        .or_default();
    sources.retain(|source| source.end_time > now);
    sources.push(ScheduledAudioSource { source, end_time });
    let schedule_state_elapsed_us = elapsed_us_since(&schedule_state_started_at);

    if !record_diagnostics {
        return Ok(None);
    }

    Ok(Some(ScheduleAudioTiming {
        frames,
        channels,
        total_elapsed_us: elapsed_us_since(&started_at),
        create_buffer_elapsed_us,
        copy_to_elapsed_us,
        copy_channel_elapsed_us,
        source_setup_elapsed_us,
        source_start_elapsed_us,
        schedule_state_elapsed_us,
        scheduled_sources: sources.len(),
    }))
}

fn should_warn_playback_schedule(
    inner: &mut AudioPlaybackInner,
    sender_user_id: &str,
    now: f64,
) -> bool {
    let last_warning_at = inner
        .playback_schedule_warning_at
        .get(sender_user_id)
        .copied()
        .unwrap_or(f64::NEG_INFINITY);
    if now - last_warning_at < PLAYBACK_SCHEDULE_WARNING_INTERVAL_SECONDS {
        return false;
    }

    inner
        .playback_schedule_warning_at
        .insert(sender_user_id.to_owned(), now);
    true
}

fn copy_options(plane_index: u32) -> JsValue {
    let options = Object::new();
    set_property(&options, "format", &JsValue::from_str("f32-planar"));
    set_property(
        &options,
        "planeIndex",
        &JsValue::from_f64(f64::from(plane_index)),
    );
    options.into()
}

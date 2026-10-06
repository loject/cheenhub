//! Browser playback decoding and scheduling pipeline.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::{debug, warn};
use js_sys::{Float32Array, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use web_sys::{AudioContext, GainNode};
use web_time::Instant;

use super::AudioPlaybackInner;
use super::browser_bindings::{AudioData, AudioDecoder, EncodedAudioChunk};
use super::browser_diagnostics::{
    DecodeOutputTiming, ScheduleAudioTiming, diagnostics_enabled, elapsed_us_since,
};
use super::browser_helpers::{js_error_message, set_property};
use super::playback_schedule::{schedule_pcm, should_warn_playback_schedule};
use crate::features::audio_playback::backend::VoiceFrame;
use crate::features::audio_playback::playout_timing::{
    PlaybackPause, scheduled_target_depth_seconds,
};
use crate::features::audio_playback::time_scale::{self, VoiceTimeCompressor};

/// Порог, выше которого коэффициент сжатия считается отличным от единицы.
const COMPRESSED_RATE_THRESHOLD: f64 = 1.0 + 1.0e-4;

pub(super) struct SenderPlayback {
    pub(super) decoder: AudioDecoder,
    pub(super) gain_node: GainNode,
    /// Компрессоры времени по каналам: сжимают звук, когда буфер отстаёт от сети.
    /// Отменяемое ожидание доигрывания хвоста текущего отправителя.
    pub(super) pause: PlaybackPause,
    pub(super) time_compressors: RefCell<Vec<Rc<RefCell<VoiceTimeCompressor>>>>,
    _output_closure: Closure<dyn FnMut(AudioData)>,
    _error_closure: Closure<dyn FnMut(JsValue)>,
}

impl SenderPlayback {
    /// Возвращает компрессор времени для канала, создавая его при первом обращении.
    pub(super) fn time_compressor(
        &self,
        channel: usize,
        sample_rate_hz: u32,
    ) -> Rc<RefCell<VoiceTimeCompressor>> {
        let mut compressors = self.time_compressors.borrow_mut();
        while compressors.len() <= channel {
            compressors.push(Rc::new(RefCell::new(VoiceTimeCompressor::new(
                sample_rate_hz,
            ))));
        }

        compressors[channel].clone()
    }
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
        time_compressors: RefCell::new(Vec::new()),
        pause: PlaybackPause::default(),
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
    let compression_now = context.current_time();
    let time_compressors = inner
        .borrow()
        .senders
        .get(sender_user_id)
        .map(|sender| {
            (0..channels)
                .map(|channel| sender.time_compressor(channel as usize, sample_rate as u32))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let (applied_rate, pending_content_seconds) = time_compressors
        .first()
        .map(|compressor| {
            let compressor = compressor.borrow();
            (
                compressor.rate(),
                compressor.pending_content_samples() as f64 / f64::from(sample_rate),
            )
        })
        .unwrap_or((1.0, 0.0));
    let compression_rate = catch_up_rate(
        &inner.borrow(),
        sender_user_id,
        compression_now,
        applied_rate,
        pending_content_seconds,
    );
    let compressing = compression_rate > COMPRESSED_RATE_THRESHOLD;
    if let Some(first_compressor) = time_compressors.first() {
        let mut compressor = first_compressor.borrow_mut();
        let was_compressing = compressor.rate() > COMPRESSED_RATE_THRESHOLD;
        compressor.set_rate(compression_rate);
        for compressor in time_compressors.iter().skip(1) {
            compressor.borrow_mut().set_rate(compression_rate);
        }
        if was_compressing != compressing {
            let depth_ms = playout_depth_ms(&inner.borrow(), sender_user_id, compression_now);
            debug!(
                %sender_user_id,
                compression_rate,
                playout_depth_ms = depth_ms,
                "inbound voice playback buffer catch-up state changed"
            );
        }
    }
    if compressing
        && should_warn_playback_schedule(&mut inner.borrow_mut(), sender_user_id, compression_now)
    {
        warn!(
            %sender_user_id,
            compression_rate,
            playout_depth_ms = playout_depth_ms(&inner.borrow(), sender_user_id, compression_now),
            "inbound voice playback buffer is catching up"
        );
    }

    let mut copy_to_elapsed_us = 0_u128;
    let mut planes: Vec<Vec<f32>> = Vec::with_capacity(channels as usize);
    for channel in 0..channels {
        let samples = Float32Array::new_with_length(frames);
        let copy_to_started_at = record_diagnostics.then(Instant::now);
        audio.copy_to(&samples, &copy_options(channel))?;
        copy_to_elapsed_us =
            copy_to_elapsed_us.saturating_add(elapsed_us_since(&copy_to_started_at));
        let plane = samples.to_vec();
        let plane = match time_compressors.get(channel as usize) {
            Some(compressor) => compressor.borrow_mut().process(&plane),
            None => plane,
        };
        planes.push(plane);
    }
    super::web_pause::arm_playback_pause(
        context,
        inner,
        sender_user_id,
        (f64::from(frames) / f64::from(sample_rate) * 1_000_000.0).round() as u32,
        sample_rate,
    );
    let mut timing = schedule_pcm(
        context,
        inner,
        sender_user_id,
        &planes,
        sample_rate,
        record_diagnostics,
    )?;
    if let Some(timing) = timing.as_mut() {
        timing.frames = frames;
        timing.copy_to_elapsed_us = copy_to_elapsed_us;
        timing.total_elapsed_us = elapsed_us_since(&started_at);
    }
    Ok(timing)
}

/// Возвращает глубину запланированного буфера воспроизведения отправителя в миллисекундах.
fn playout_depth_ms(inner: &AudioPlaybackInner, sender_user_id: &str, now: f64) -> f64 {
    inner
        .scheduled_until
        .get(sender_user_id)
        .map_or(0.0, |scheduled_until| (scheduled_until - now) * 1000.0)
}

/// Возвращает коэффициент сжатия времени по избытку буфера воспроизведения.
///
/// Отставание считается в единицах исходного контента: уже запланированные буферы
/// хранят сжатый звук, поэтому их глубина умножается на применённый коэффициент, и
/// к ней добавляется контент, ещё не выведенный компрессором.
fn catch_up_rate(
    inner: &AudioPlaybackInner,
    sender_user_id: &str,
    now: f64,
    applied_rate: f64,
    pending_content_seconds: f64,
) -> f64 {
    let Some(scheduled_until) = inner.scheduled_until.get(sender_user_id) else {
        return 1.0;
    };

    let buffered_content = (*scheduled_until - now) * applied_rate + pending_content_seconds;
    let target_depth = scheduled_target_depth_seconds(inner.jitter_buffer_us);
    time_scale::catch_up_rate(buffered_content - target_depth)
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

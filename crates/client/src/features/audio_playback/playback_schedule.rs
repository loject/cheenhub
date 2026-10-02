//! Планирование готовых PCM-плоскостей в браузерном AudioContext.

use dioxus::prelude::{debug, warn};
use js_sys::Float32Array;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsValue;
use web_sys::{AudioBufferSourceNode, AudioContext};
use web_time::Instant;

use super::AudioPlaybackInner;
use super::browser_diagnostics::{ScheduleAudioTiming, elapsed_us_since};
use crate::features::audio_playback::playout_timing::scheduled_start_seconds;

/// Источник PCM с временем окончания для очистки проигранных буферов.
pub(super) struct ScheduledAudioSource {
    /// Запланированный источник браузерного аудио.
    pub(super) source: AudioBufferSourceNode,
    /// Время окончания в шкале AudioContext.
    pub(super) end_time: f64,
}

const PLAYBACK_SCHEDULE_WARNING_INTERVAL_SECONDS: f64 = 5.0;

/// Планирует PCM обычного кадра или остатка после паузы тем же путём вывода.
pub(super) fn schedule_pcm(
    context: &AudioContext,
    inner: &Rc<RefCell<AudioPlaybackInner>>,
    sender_user_id: &str,
    planes: &[Vec<f32>],
    sample_rate: f32,
    record_diagnostics: bool,
) -> Result<Option<ScheduleAudioTiming>, JsValue> {
    let started_at = record_diagnostics.then(Instant::now);
    let create_buffer_started_at = record_diagnostics.then(Instant::now);
    let copy_to_elapsed_us = 0;
    let mut copy_channel_elapsed_us = 0_u128;
    let compressed_frames = planes.first().map_or(0, Vec::len) as u32;
    let frames = compressed_frames;
    let channels = planes.len() as u32;
    if compressed_frames == 0 {
        return Ok(None);
    }
    let buffer = context.create_buffer(channels, compressed_frames, sample_rate)?;
    let create_buffer_elapsed_us = elapsed_us_since(&create_buffer_started_at);

    for (channel, plane) in planes.iter().enumerate() {
        let copy_channel_started_at = record_diagnostics.then(Instant::now);
        buffer.copy_to_channel_with_f32_array(
            &Float32Array::from(plane.as_slice()),
            channel as i32,
        )?;
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
    let start_at = scheduled_start_seconds(now, previous_until, inner.jitter_buffer_us);
    if previous_until.is_none_or(|until| until <= now) {
        let buffer_ms = (start_at - now) * 1000.0;
        debug!(%sender_user_id, buffer_ms, "priming inbound voice playback buffer");
        if let Some(previous_until) = previous_until
            && should_warn_playback_schedule(&mut inner, sender_user_id, now)
        {
            warn!(
                %sender_user_id,
                underrun_ms = (now - previous_until) * 1000.0,
                buffer_ms,
                "inbound voice playback underrun"
            );
        }
    }
    let duration = f64::from(compressed_frames) / f64::from(sample_rate);
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

pub(super) fn should_warn_playback_schedule(
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

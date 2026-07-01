//! Browser audio playback diagnostics.

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use std::collections::{HashMap, VecDeque};

use dioxus::prelude::warn;
use web_time::{Instant, SystemTime, UNIX_EPOCH};

use super::AUDIO_PLAYBACK_WARNING_INTERVAL_MS;

const MAX_PENDING_DECODE_OUTPUTS: usize = 512;

pub(super) fn diagnostics_enabled() -> bool {
    cfg!(feature = "browser-media-diagnostics")
}

pub(super) fn elapsed_us_since(started_at: &Option<Instant>) -> u128 {
    started_at
        .as_ref()
        .map(|started_at| started_at.elapsed().as_micros())
        .unwrap_or_default()
}

pub(super) struct AudioPlaybackDiagnostics {
    pending_decode_outputs: HashMap<String, VecDeque<PendingDecodeFrame>>,
    decode_input_profile: DecodeInputProfile,
    decode_output_profile: DecodeOutputProfile,
    schedule_profile: ScheduleProfile,
}

impl AudioPlaybackDiagnostics {
    pub(super) fn new() -> Self {
        Self {
            pending_decode_outputs: HashMap::new(),
            decode_input_profile: DecodeInputProfile::default(),
            decode_output_profile: DecodeOutputProfile::default(),
            schedule_profile: ScheduleProfile::default(),
        }
    }

    pub(super) fn remove_sender(&mut self, sender_user_id: &str) {
        self.pending_decode_outputs.remove(sender_user_id);
    }

    pub(super) fn record_decode_input(&mut self, sender_user_id: &str, timing: DecodeInputTiming) {
        if !diagnostics_enabled() {
            return;
        }

        let now_ms = browser_now_ms();
        let pending = self
            .pending_decode_outputs
            .entry(sender_user_id.to_owned())
            .or_default();
        if pending.len() >= MAX_PENDING_DECODE_OUTPUTS {
            pending.pop_front();
            self.decode_output_profile.dropped_pending_inputs = self
                .decode_output_profile
                .dropped_pending_inputs
                .saturating_add(1);
        }
        pending.push_back(PendingDecodeFrame {
            timestamp_us: timing.timestamp_us,
            enqueued_at: Instant::now(),
        });

        self.decode_input_profile.record(&timing);
        if self.decode_input_profile.window_start_ms == 0 {
            self.decode_input_profile.window_start_ms = now_ms;
            return;
        }
        if now_ms.saturating_sub(self.decode_input_profile.window_start_ms)
            < AUDIO_PLAYBACK_WARNING_INTERVAL_MS
        {
            return;
        }

        let count = self.decode_input_profile.count.max(1);
        warn!(
            count = self.decode_input_profile.count,
            window_ms = now_ms.saturating_sub(self.decode_input_profile.window_start_ms),
            payload_bytes = self.decode_input_profile.payload_bytes,
            total_elapsed_ms = self.decode_input_profile.total_elapsed_us / 1_000,
            context_elapsed_ms = self.decode_input_profile.context_elapsed_us / 1_000,
            chunk_elapsed_ms = self.decode_input_profile.chunk_elapsed_us / 1_000,
            sender_elapsed_ms = self.decode_input_profile.sender_elapsed_us / 1_000,
            decode_elapsed_ms = self.decode_input_profile.decode_elapsed_us / 1_000,
            avg_total_us = self.decode_input_profile.total_elapsed_us / u128::from(count),
            max_total_us = self.decode_input_profile.max_total_elapsed_us,
            max_queue_size = self.decode_input_profile.max_queue_size,
            pending_outputs = pending.len(),
            "browser audio decode input profile"
        );
        self.decode_input_profile = DecodeInputProfile {
            window_start_ms: now_ms,
            ..Default::default()
        };
    }

    pub(super) fn record_decode_output(
        &mut self,
        sender_user_id: &str,
        timing: DecodeOutputTiming,
    ) {
        if !diagnostics_enabled() {
            return;
        }

        let decode_latency_us = self.take_decode_latency_us(sender_user_id, timing.timestamp_us);
        let now_ms = browser_now_ms();
        self.decode_output_profile
            .record(&timing, decode_latency_us);
        if self.decode_output_profile.window_start_ms == 0 {
            self.decode_output_profile.window_start_ms = now_ms;
            return;
        }
        if now_ms.saturating_sub(self.decode_output_profile.window_start_ms)
            < AUDIO_PLAYBACK_WARNING_INTERVAL_MS
        {
            return;
        }

        let count = self.decode_output_profile.count.max(1);
        warn!(
            count = self.decode_output_profile.count,
            window_ms = now_ms.saturating_sub(self.decode_output_profile.window_start_ms),
            frames = self.decode_output_profile.frames,
            total_elapsed_ms = self.decode_output_profile.total_elapsed_us / 1_000,
            schedule_elapsed_ms = self.decode_output_profile.schedule_elapsed_us / 1_000,
            close_elapsed_ms = self.decode_output_profile.close_elapsed_us / 1_000,
            decode_latency_ms = self.decode_output_profile.decode_latency_us / 1_000,
            avg_total_us = self.decode_output_profile.total_elapsed_us / u128::from(count),
            max_total_us = self.decode_output_profile.max_total_elapsed_us,
            max_decode_latency_us = self.decode_output_profile.max_decode_latency_us,
            missing_latency_count = self.decode_output_profile.missing_latency_count,
            dropped_pending_inputs = self.decode_output_profile.dropped_pending_inputs,
            max_channels = self.decode_output_profile.max_channels,
            "browser audio decoded output profile"
        );
        self.decode_output_profile = DecodeOutputProfile {
            window_start_ms: now_ms,
            ..Default::default()
        };
    }

    pub(super) fn record_schedule(&mut self, _sender_user_id: &str, timing: ScheduleAudioTiming) {
        if !diagnostics_enabled() {
            return;
        }

        let now_ms = browser_now_ms();
        self.schedule_profile.record(&timing);
        if self.schedule_profile.window_start_ms == 0 {
            self.schedule_profile.window_start_ms = now_ms;
            return;
        }
        if now_ms.saturating_sub(self.schedule_profile.window_start_ms)
            < AUDIO_PLAYBACK_WARNING_INTERVAL_MS
        {
            return;
        }

        let count = self.schedule_profile.count.max(1);
        warn!(
            count = self.schedule_profile.count,
            window_ms = now_ms.saturating_sub(self.schedule_profile.window_start_ms),
            frames = self.schedule_profile.frames,
            total_elapsed_ms = self.schedule_profile.total_elapsed_us / 1_000,
            create_buffer_elapsed_ms = self.schedule_profile.create_buffer_elapsed_us / 1_000,
            copy_to_elapsed_ms = self.schedule_profile.copy_to_elapsed_us / 1_000,
            copy_channel_elapsed_ms = self.schedule_profile.copy_channel_elapsed_us / 1_000,
            source_setup_elapsed_ms = self.schedule_profile.source_setup_elapsed_us / 1_000,
            source_start_elapsed_ms = self.schedule_profile.source_start_elapsed_us / 1_000,
            schedule_state_elapsed_ms = self.schedule_profile.schedule_state_elapsed_us / 1_000,
            avg_total_us = self.schedule_profile.total_elapsed_us / u128::from(count),
            max_total_us = self.schedule_profile.max_total_elapsed_us,
            max_scheduled_sources = self.schedule_profile.max_scheduled_sources,
            max_channels = self.schedule_profile.max_channels,
            "browser audio playback schedule profile"
        );
        self.schedule_profile = ScheduleProfile {
            window_start_ms: now_ms,
            ..Default::default()
        };
    }

    fn take_decode_latency_us(&mut self, sender_user_id: &str, timestamp_us: u64) -> Option<u128> {
        let pending = self.pending_decode_outputs.get_mut(sender_user_id)?;
        let position = pending
            .iter()
            .position(|frame| frame.timestamp_us == timestamp_us)?;
        let frame = pending.remove(position)?;
        Some(frame.enqueued_at.elapsed().as_micros())
    }
}

pub(super) struct DecodeInputTiming {
    pub(super) timestamp_us: u64,
    pub(super) payload_bytes: usize,
    pub(super) total_elapsed_us: u128,
    pub(super) context_elapsed_us: u128,
    pub(super) chunk_elapsed_us: u128,
    pub(super) sender_elapsed_us: u128,
    pub(super) decode_elapsed_us: u128,
    pub(super) queue_size: u32,
}

pub(super) struct DecodeOutputTiming {
    pub(super) timestamp_us: u64,
    pub(super) frames: u32,
    pub(super) channels: u32,
    pub(super) total_elapsed_us: u128,
    pub(super) schedule_elapsed_us: u128,
    pub(super) close_elapsed_us: u128,
}

#[derive(Clone)]
pub(super) struct ScheduleAudioTiming {
    pub(super) frames: u32,
    pub(super) channels: u32,
    pub(super) total_elapsed_us: u128,
    pub(super) create_buffer_elapsed_us: u128,
    pub(super) copy_to_elapsed_us: u128,
    pub(super) copy_channel_elapsed_us: u128,
    pub(super) source_setup_elapsed_us: u128,
    pub(super) source_start_elapsed_us: u128,
    pub(super) schedule_state_elapsed_us: u128,
    pub(super) scheduled_sources: usize,
}

struct PendingDecodeFrame {
    timestamp_us: u64,
    enqueued_at: Instant,
}

#[derive(Default)]
struct DecodeInputProfile {
    window_start_ms: u64,
    count: u64,
    payload_bytes: u64,
    total_elapsed_us: u128,
    context_elapsed_us: u128,
    chunk_elapsed_us: u128,
    sender_elapsed_us: u128,
    decode_elapsed_us: u128,
    max_total_elapsed_us: u128,
    max_queue_size: u32,
}

impl DecodeInputProfile {
    fn record(&mut self, timing: &DecodeInputTiming) {
        self.count = self.count.saturating_add(1);
        self.payload_bytes = self
            .payload_bytes
            .saturating_add(timing.payload_bytes as u64);
        self.total_elapsed_us = self
            .total_elapsed_us
            .saturating_add(timing.total_elapsed_us);
        self.context_elapsed_us = self
            .context_elapsed_us
            .saturating_add(timing.context_elapsed_us);
        self.chunk_elapsed_us = self
            .chunk_elapsed_us
            .saturating_add(timing.chunk_elapsed_us);
        self.sender_elapsed_us = self
            .sender_elapsed_us
            .saturating_add(timing.sender_elapsed_us);
        self.decode_elapsed_us = self
            .decode_elapsed_us
            .saturating_add(timing.decode_elapsed_us);
        self.max_total_elapsed_us = self.max_total_elapsed_us.max(timing.total_elapsed_us);
        self.max_queue_size = self.max_queue_size.max(timing.queue_size);
    }
}

#[derive(Default)]
struct DecodeOutputProfile {
    window_start_ms: u64,
    count: u64,
    frames: u64,
    total_elapsed_us: u128,
    schedule_elapsed_us: u128,
    close_elapsed_us: u128,
    decode_latency_us: u128,
    max_total_elapsed_us: u128,
    max_decode_latency_us: u128,
    missing_latency_count: u64,
    dropped_pending_inputs: u64,
    max_channels: u32,
}

impl DecodeOutputProfile {
    fn record(&mut self, timing: &DecodeOutputTiming, decode_latency_us: Option<u128>) {
        self.count = self.count.saturating_add(1);
        self.frames = self.frames.saturating_add(u64::from(timing.frames));
        self.total_elapsed_us = self
            .total_elapsed_us
            .saturating_add(timing.total_elapsed_us);
        self.schedule_elapsed_us = self
            .schedule_elapsed_us
            .saturating_add(timing.schedule_elapsed_us);
        self.close_elapsed_us = self
            .close_elapsed_us
            .saturating_add(timing.close_elapsed_us);
        if let Some(decode_latency_us) = decode_latency_us {
            self.decode_latency_us = self.decode_latency_us.saturating_add(decode_latency_us);
            self.max_decode_latency_us = self.max_decode_latency_us.max(decode_latency_us);
        } else {
            self.missing_latency_count = self.missing_latency_count.saturating_add(1);
        }
        self.max_total_elapsed_us = self.max_total_elapsed_us.max(timing.total_elapsed_us);
        self.max_channels = self.max_channels.max(timing.channels);
    }
}

#[derive(Default)]
struct ScheduleProfile {
    window_start_ms: u64,
    count: u64,
    frames: u64,
    total_elapsed_us: u128,
    create_buffer_elapsed_us: u128,
    copy_to_elapsed_us: u128,
    copy_channel_elapsed_us: u128,
    source_setup_elapsed_us: u128,
    source_start_elapsed_us: u128,
    schedule_state_elapsed_us: u128,
    max_total_elapsed_us: u128,
    max_scheduled_sources: usize,
    max_channels: u32,
}

impl ScheduleProfile {
    fn record(&mut self, timing: &ScheduleAudioTiming) {
        self.count = self.count.saturating_add(1);
        self.frames = self.frames.saturating_add(u64::from(timing.frames));
        self.total_elapsed_us = self
            .total_elapsed_us
            .saturating_add(timing.total_elapsed_us);
        self.create_buffer_elapsed_us = self
            .create_buffer_elapsed_us
            .saturating_add(timing.create_buffer_elapsed_us);
        self.copy_to_elapsed_us = self
            .copy_to_elapsed_us
            .saturating_add(timing.copy_to_elapsed_us);
        self.copy_channel_elapsed_us = self
            .copy_channel_elapsed_us
            .saturating_add(timing.copy_channel_elapsed_us);
        self.source_setup_elapsed_us = self
            .source_setup_elapsed_us
            .saturating_add(timing.source_setup_elapsed_us);
        self.source_start_elapsed_us = self
            .source_start_elapsed_us
            .saturating_add(timing.source_start_elapsed_us);
        self.schedule_state_elapsed_us = self
            .schedule_state_elapsed_us
            .saturating_add(timing.schedule_state_elapsed_us);
        self.max_total_elapsed_us = self.max_total_elapsed_us.max(timing.total_elapsed_us);
        self.max_scheduled_sources = self.max_scheduled_sources.max(timing.scheduled_sources);
        self.max_channels = self.max_channels.max(timing.channels);
    }
}

fn browser_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

//! Browser microphone diagnostics.

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use dioxus::prelude::warn;
use web_time::{Instant, SystemTime, UNIX_EPOCH};

const MICROPHONE_WORKLET_DELIVERY_WARN_MS: u64 = 120;
const MICROPHONE_WORKLET_PROCESSING_WARN_US: u128 = 12_000;
const MICROPHONE_WARNING_INTERVAL_MS: u64 = 5_000;
const MAX_PENDING_ENCODER_INPUTS: usize = 512;

pub(super) fn enabled() -> bool {
    cfg!(feature = "browser-media-diagnostics")
}

pub(super) struct MicrophoneWorkletDiagnostics {
    encoder_queue_warning_ms: Rc<Cell<u64>>,
    processing_warning_ms: Rc<Cell<u64>>,
    chunk_timing: Rc<RefCell<MicrophoneChunkTiming>>,
    processing_profile: Rc<RefCell<WorkletProcessingProfile>>,
}

impl MicrophoneWorkletDiagnostics {
    pub(super) fn new() -> Self {
        Self {
            encoder_queue_warning_ms: Rc::new(Cell::new(0)),
            processing_warning_ms: Rc::new(Cell::new(0)),
            chunk_timing: Rc::new(RefCell::new(MicrophoneChunkTiming::default())),
            processing_profile: Rc::new(RefCell::new(WorkletProcessingProfile::default())),
        }
    }

    pub(super) fn should_emit_encoder_queue_warning(&self) -> bool {
        if !enabled() {
            return false;
        }

        should_emit_cell_warning(
            &self.encoder_queue_warning_ms,
            browser_now_ms(),
            MICROPHONE_WARNING_INTERVAL_MS,
        )
    }

    pub(super) fn warn_if_delivery_is_late(&self, timing: WorkletDeliveryTiming) {
        if !enabled() {
            return;
        }

        let now_ms = browser_now_ms();
        let mut chunk_timing = self.chunk_timing.borrow_mut();
        let first_timestamp_us = match chunk_timing.first_timestamp_us {
            Some(timestamp_us) => timestamp_us,
            None => {
                chunk_timing.first_timestamp_us = Some(timing.timestamp_us);
                chunk_timing.first_wall_ms = now_ms;
                chunk_timing.last_timestamp_us = Some(timing.timestamp_us);
                return;
            }
        };

        let expected_wall_ms = chunk_timing.first_wall_ms.saturating_add(
            timing
                .timestamp_us
                .saturating_sub(first_timestamp_us)
                .checked_div(1_000)
                .unwrap_or_default(),
        );
        let delivery_late_ms = now_ms.saturating_sub(expected_wall_ms);
        let previous_timestamp_us = chunk_timing.last_timestamp_us.replace(timing.timestamp_us);
        let timestamp_gap_us = previous_timestamp_us
            .map(|previous_timestamp_us| timing.timestamp_us.saturating_sub(previous_timestamp_us))
            .unwrap_or_default();
        let timestamp_gap_ms = timestamp_gap_us.checked_div(1_000).unwrap_or_default();
        let expected_gap_ms = u64::from(timing.duration_us)
            .checked_div(1_000)
            .unwrap_or_default();
        if delivery_late_ms < MICROPHONE_WORKLET_DELIVERY_WARN_MS {
            return;
        }
        if chunk_timing.last_warning_ms != 0
            && now_ms.saturating_sub(chunk_timing.last_warning_ms) < MICROPHONE_WARNING_INTERVAL_MS
        {
            return;
        }

        chunk_timing.last_warning_ms = now_ms;
        warn!(
            delivery_late_ms,
            timestamp_gap_ms,
            expected_gap_ms,
            timestamp_us = timing.timestamp_us,
            "microphone audio worklet chunk reached main thread late"
        );
    }

    pub(super) fn record_processing(&self, timing: WorkletProcessingTiming) {
        if !enabled() {
            return;
        }

        self.record_processing_profile(&timing);
        self.warn_if_processing_is_slow(&timing);
    }

    pub(super) fn record_processor_profile(&self, timing: WorkletProcessorProfileTiming) {
        if !enabled() {
            return;
        }

        let count = timing.process_count.max(1);
        warn!(
            process_count = timing.process_count,
            chunk_count = timing.chunk_count,
            input_empty_count = timing.input_empty_count,
            frames = timing.frames,
            window_ms = timing.window_ms,
            total_elapsed_ms = timing.total_elapsed_us / 1_000,
            clear_elapsed_ms = timing.clear_elapsed_us / 1_000,
            copy_elapsed_ms = timing.copy_elapsed_us / 1_000,
            post_elapsed_ms = timing.post_elapsed_us / 1_000,
            allocate_elapsed_ms = timing.allocate_elapsed_us / 1_000,
            avg_process_us = timing.total_elapsed_us / u128::from(count),
            max_process_us = timing.max_process_us,
            max_channels = timing.max_channels,
            "microphone audio worklet processor profile"
        );
    }

    fn record_processing_profile(&self, timing: &WorkletProcessingTiming) {
        let now_ms = browser_now_ms();
        let mut profile = self.processing_profile.borrow_mut();
        profile.record(timing);
        if profile.window_start_ms == 0 {
            profile.window_start_ms = now_ms;
            return;
        }
        if now_ms.saturating_sub(profile.window_start_ms) < MICROPHONE_WARNING_INTERVAL_MS {
            return;
        }

        let count = profile.count.max(1);
        warn!(
            count = profile.count,
            active_count = profile.active_count,
            window_ms = now_ms.saturating_sub(profile.window_start_ms),
            total_elapsed_ms = profile.total_elapsed_us / 1_000,
            parse_elapsed_ms = profile.parse_elapsed_us / 1_000,
            level_elapsed_ms = profile.level_elapsed_us / 1_000,
            audio_data_elapsed_ms = profile.audio_data_elapsed_us / 1_000,
            encode_elapsed_ms = profile.encode_elapsed_us / 1_000,
            close_elapsed_ms = profile.close_elapsed_us / 1_000,
            avg_total_us = profile.total_elapsed_us / u128::from(count),
            max_total_us = profile.max_total_elapsed_us,
            max_queue_size = profile.max_queue_size,
            "microphone audio worklet main thread processing profile"
        );
        *profile = WorkletProcessingProfile {
            window_start_ms: now_ms,
            ..Default::default()
        };
    }

    fn warn_if_processing_is_slow(&self, timing: &WorkletProcessingTiming) {
        if timing.total_elapsed_us < MICROPHONE_WORKLET_PROCESSING_WARN_US {
            return;
        }
        if !should_emit_cell_warning(
            &self.processing_warning_ms,
            browser_now_ms(),
            MICROPHONE_WARNING_INTERVAL_MS,
        ) {
            return;
        }

        warn!(
            timestamp_us = timing.timestamp_us,
            duration_us = timing.duration_us,
            active = timing.active,
            total_elapsed_us = timing.total_elapsed_us,
            parse_elapsed_us = timing.parse_elapsed_us,
            level_elapsed_us = timing.level_elapsed_us,
            audio_data_elapsed_us = timing.audio_data_elapsed_us,
            encode_elapsed_us = timing.encode_elapsed_us,
            close_elapsed_us = timing.close_elapsed_us,
            queue_size = timing.queue_size,
            "microphone audio worklet chunk processing is slow on main thread"
        );
    }
}

pub(super) struct MicrophoneEncoderOutputDiagnostics {
    pending_inputs: Rc<RefCell<VecDeque<PendingEncoderInput>>>,
    output_profile: Rc<RefCell<EncoderOutputProfile>>,
}

impl MicrophoneEncoderOutputDiagnostics {
    pub(super) fn new() -> Self {
        Self {
            pending_inputs: Rc::new(RefCell::new(VecDeque::new())),
            output_profile: Rc::new(RefCell::new(EncoderOutputProfile::default())),
        }
    }

    pub(super) fn record_input(&self, timing: EncoderInputTiming) {
        if !enabled() {
            return;
        }

        let mut pending_inputs = self.pending_inputs.borrow_mut();
        if pending_inputs.len() >= MAX_PENDING_ENCODER_INPUTS {
            pending_inputs.pop_front();
            let mut profile = self.output_profile.borrow_mut();
            profile.dropped_input_count = profile.dropped_input_count.saturating_add(1);
        }
        pending_inputs.push_back(PendingEncoderInput {
            duration_us: u64::from(timing.duration_us),
            enqueued_at: Instant::now(),
        });
    }

    pub(super) fn record_output(&self, timing: EncoderOutputTiming) {
        if !enabled() {
            return;
        }

        let encode_latency = self.take_encode_latency(&timing);
        let pending_input_count = self.pending_inputs.borrow().len();
        let now_ms = browser_now_ms();
        let mut profile = self.output_profile.borrow_mut();
        profile.record(&timing, encode_latency);
        if profile.window_start_ms == 0 {
            profile.window_start_ms = now_ms;
            return;
        }
        if now_ms.saturating_sub(profile.window_start_ms) < MICROPHONE_WARNING_INTERVAL_MS {
            return;
        }

        let count = profile.count.max(1);
        warn!(
            count = profile.count,
            window_ms = now_ms.saturating_sub(profile.window_start_ms),
            payload_bytes = profile.payload_bytes,
            total_elapsed_ms = profile.total_elapsed_us / 1_000,
            copy_chunk_elapsed_ms = profile.copy_chunk_elapsed_us / 1_000,
            vec_copy_elapsed_ms = profile.vec_copy_elapsed_us / 1_000,
            on_frame_elapsed_ms = profile.on_frame_elapsed_us / 1_000,
            encode_latency_ms = profile.encode_latency_us / 1_000,
            avg_total_us = profile.total_elapsed_us / u128::from(count),
            max_total_us = profile.max_total_elapsed_us,
            max_encode_latency_us = profile.max_encode_latency_us,
            missing_latency_count = profile.missing_latency_count,
            dropped_input_count = profile.dropped_input_count,
            consumed_input_count = profile.consumed_input_count,
            max_inputs_per_output = profile.max_inputs_per_output,
            pending_input_count,
            "microphone WebCodecs output callback profile"
        );
        *profile = EncoderOutputProfile {
            window_start_ms: now_ms,
            ..Default::default()
        };
    }

    fn take_encode_latency(&self, timing: &EncoderOutputTiming) -> Option<EncoderLatencyTiming> {
        let mut pending_inputs = self.pending_inputs.borrow_mut();
        let first_input = pending_inputs.pop_front()?;
        let mut consumed_inputs = 1_u64;
        let mut consumed_duration_us = first_input.duration_us;
        while consumed_duration_us < u64::from(timing.duration_us) {
            let Some(input) = pending_inputs.pop_front() else {
                break;
            };
            consumed_duration_us = consumed_duration_us.saturating_add(input.duration_us);
            consumed_inputs = consumed_inputs.saturating_add(1);
        }
        Some(EncoderLatencyTiming {
            elapsed_us: first_input.enqueued_at.elapsed().as_micros(),
            consumed_inputs,
        })
    }
}

pub(super) struct WorkletDeliveryTiming {
    pub(super) timestamp_us: u64,
    pub(super) duration_us: u32,
}

pub(super) struct WorkletProcessingTiming {
    pub(super) timestamp_us: u64,
    pub(super) duration_us: u32,
    pub(super) active: bool,
    pub(super) total_elapsed_us: u128,
    pub(super) parse_elapsed_us: u128,
    pub(super) level_elapsed_us: u128,
    pub(super) audio_data_elapsed_us: u128,
    pub(super) encode_elapsed_us: u128,
    pub(super) close_elapsed_us: u128,
    pub(super) queue_size: u32,
}

pub(super) struct WorkletProcessorProfileTiming {
    pub(super) process_count: u64,
    pub(super) chunk_count: u64,
    pub(super) input_empty_count: u64,
    pub(super) frames: u64,
    pub(super) window_ms: u64,
    pub(super) total_elapsed_us: u128,
    pub(super) clear_elapsed_us: u128,
    pub(super) copy_elapsed_us: u128,
    pub(super) post_elapsed_us: u128,
    pub(super) allocate_elapsed_us: u128,
    pub(super) max_process_us: u128,
    pub(super) max_channels: u32,
}

pub(super) struct EncoderOutputTiming {
    pub(super) duration_us: u32,
    pub(super) payload_bytes: u32,
    pub(super) total_elapsed_us: u128,
    pub(super) copy_chunk_elapsed_us: u128,
    pub(super) vec_copy_elapsed_us: u128,
    pub(super) on_frame_elapsed_us: u128,
}

pub(super) struct EncoderInputTiming {
    pub(super) duration_us: u32,
}

struct PendingEncoderInput {
    duration_us: u64,
    enqueued_at: Instant,
}

struct EncoderLatencyTiming {
    elapsed_us: u128,
    consumed_inputs: u64,
}

#[derive(Default)]
struct MicrophoneChunkTiming {
    first_timestamp_us: Option<u64>,
    first_wall_ms: u64,
    last_timestamp_us: Option<u64>,
    last_warning_ms: u64,
}

#[derive(Default)]
struct WorkletProcessingProfile {
    window_start_ms: u64,
    count: u64,
    active_count: u64,
    total_elapsed_us: u128,
    parse_elapsed_us: u128,
    level_elapsed_us: u128,
    audio_data_elapsed_us: u128,
    encode_elapsed_us: u128,
    close_elapsed_us: u128,
    max_total_elapsed_us: u128,
    max_queue_size: u32,
}

impl WorkletProcessingProfile {
    fn record(&mut self, timing: &WorkletProcessingTiming) {
        self.count = self.count.saturating_add(1);
        if timing.active {
            self.active_count = self.active_count.saturating_add(1);
        }
        self.total_elapsed_us = self
            .total_elapsed_us
            .saturating_add(timing.total_elapsed_us);
        self.parse_elapsed_us = self
            .parse_elapsed_us
            .saturating_add(timing.parse_elapsed_us);
        self.level_elapsed_us = self
            .level_elapsed_us
            .saturating_add(timing.level_elapsed_us);
        self.audio_data_elapsed_us = self
            .audio_data_elapsed_us
            .saturating_add(timing.audio_data_elapsed_us);
        self.encode_elapsed_us = self
            .encode_elapsed_us
            .saturating_add(timing.encode_elapsed_us);
        self.close_elapsed_us = self
            .close_elapsed_us
            .saturating_add(timing.close_elapsed_us);
        self.max_total_elapsed_us = self.max_total_elapsed_us.max(timing.total_elapsed_us);
        self.max_queue_size = self.max_queue_size.max(timing.queue_size);
    }
}

#[derive(Default)]
struct EncoderOutputProfile {
    window_start_ms: u64,
    count: u64,
    payload_bytes: u64,
    total_elapsed_us: u128,
    copy_chunk_elapsed_us: u128,
    vec_copy_elapsed_us: u128,
    on_frame_elapsed_us: u128,
    encode_latency_us: u128,
    max_total_elapsed_us: u128,
    max_encode_latency_us: u128,
    missing_latency_count: u64,
    dropped_input_count: u64,
    consumed_input_count: u64,
    max_inputs_per_output: u64,
}

impl EncoderOutputProfile {
    fn record(
        &mut self,
        timing: &EncoderOutputTiming,
        encode_latency: Option<EncoderLatencyTiming>,
    ) {
        self.count = self.count.saturating_add(1);
        self.payload_bytes = self
            .payload_bytes
            .saturating_add(u64::from(timing.payload_bytes));
        self.total_elapsed_us = self
            .total_elapsed_us
            .saturating_add(timing.total_elapsed_us);
        self.copy_chunk_elapsed_us = self
            .copy_chunk_elapsed_us
            .saturating_add(timing.copy_chunk_elapsed_us);
        self.vec_copy_elapsed_us = self
            .vec_copy_elapsed_us
            .saturating_add(timing.vec_copy_elapsed_us);
        self.on_frame_elapsed_us = self
            .on_frame_elapsed_us
            .saturating_add(timing.on_frame_elapsed_us);
        if let Some(encode_latency) = encode_latency {
            self.encode_latency_us = self
                .encode_latency_us
                .saturating_add(encode_latency.elapsed_us);
            self.max_encode_latency_us = self.max_encode_latency_us.max(encode_latency.elapsed_us);
            self.consumed_input_count = self
                .consumed_input_count
                .saturating_add(encode_latency.consumed_inputs);
            self.max_inputs_per_output = self
                .max_inputs_per_output
                .max(encode_latency.consumed_inputs);
        } else {
            self.missing_latency_count = self.missing_latency_count.saturating_add(1);
        }
        self.max_total_elapsed_us = self.max_total_elapsed_us.max(timing.total_elapsed_us);
    }
}

fn should_emit_cell_warning(last_warning_ms: &Cell<u64>, now_ms: u64, interval_ms: u64) -> bool {
    let last_ms = last_warning_ms.get();
    if last_ms != 0 && now_ms.saturating_sub(last_ms) < interval_ms {
        return false;
    }

    last_warning_ms.set(now_ms);
    true
}

fn browser_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

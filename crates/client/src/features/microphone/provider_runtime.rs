//! Microphone provider runtime helpers.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use futures_channel::mpsc;
use futures_util::StreamExt;

use super::backend::{
    EncodedMicrophoneFrame, MicrophoneBackend, MicrophoneConfig, MicrophoneError,
    MicrophoneFrameCallback, MicrophoneLevel, MicrophoneSession, MicrophoneStatus,
    MicrophoneUplinkConfig,
};
use super::provider::{ActiveCapture, MicrophoneCommand};
use super::storage;

const MICROPHONE_LEVEL_UPDATE_INTERVAL_US: u64 = 33_000;

#[derive(Clone)]
pub(super) struct MicrophoneRuntime {
    pub(super) backend: Rc<dyn MicrophoneBackend>,
    pub(super) status: Signal<MicrophoneStatus>,
    pub(super) level: Signal<MicrophoneLevel>,
    pub(super) level_active: Signal<bool>,
    pub(super) session: Signal<Option<Rc<dyn MicrophoneSession>>>,
    pub(super) generation: Signal<u64>,
    pub(super) selected_input_device_id: Signal<Option<String>>,
    pub(super) input_volume_percent: Signal<u32>,
    pub(super) activation_mode: Signal<super::backend::MicrophoneActivationMode>,
    pub(super) vad_threshold_percent: Signal<u32>,
    pub(super) active_capture: Signal<ActiveCapture>,
    pub(super) active_on_frame: Signal<Option<MicrophoneFrameCallback>>,
    pub(super) active_uplink: Signal<Option<MicrophoneUplinkConfig>>,
    pub(super) target_bitrate_bps: Signal<u32>,
}

pub(super) async fn run_microphone_runtime(
    mut receiver: mpsc::UnboundedReceiver<MicrophoneCommand>,
    runtime: MicrophoneRuntime,
) {
    info!("microphone provider runtime started");
    while let Some(command) = receiver.next().await {
        match command {
            MicrophoneCommand::Start {
                operation_generation,
                on_frame,
                capture,
                uplink,
            } => {
                if operation_is_current(&runtime, operation_generation) {
                    start_capture(
                        runtime.clone(),
                        operation_generation,
                        on_frame,
                        capture,
                        uplink,
                    );
                }
            }
            MicrophoneCommand::Restart {
                operation_generation,
                on_frame,
                capture,
                uplink,
            } => {
                if operation_is_current(&runtime, operation_generation) {
                    restart_capture(
                        runtime.clone(),
                        operation_generation,
                        on_frame,
                        capture,
                        uplink,
                    );
                }
            }
            MicrophoneCommand::Stop {
                operation_generation,
            } => {
                if operation_is_current(&runtime, operation_generation) {
                    stop_capture(runtime.clone());
                }
            }
            MicrophoneCommand::SetBitrate { bitrate_bps } => {
                update_bitrate(runtime.clone(), bitrate_bps);
            }
        }
    }
    debug!("microphone provider runtime command channel closed");
}

fn start_capture(
    runtime: MicrophoneRuntime,
    operation_generation: u64,
    on_frame: MicrophoneFrameCallback,
    capture: ActiveCapture,
    uplink: Option<MicrophoneUplinkConfig>,
) {
    let previous_status = runtime.status.peek().clone();
    info!(
        ?previous_status,
        ?capture,
        has_uplink = uplink.is_some(),
        "starting microphone capture"
    );
    prepare_capture_state(&runtime, on_frame.clone(), capture, uplink.clone());
    let config = capture_config(&runtime);

    spawn(async move {
        let callbacks = microphone_callbacks(
            on_frame.clone(),
            runtime.level,
            runtime.level_active,
            runtime.status,
            uplink.clone(),
        );
        match runtime.backend.start(config, callbacks).await {
            Ok(next_session) => {
                if !operation_is_current(&runtime, operation_generation) {
                    next_session.stop_immediately();
                    if let Err(error) = next_session.stop().await {
                        warn!(%error, "failed to stop stale microphone capture after start");
                    }
                    return;
                }
                let mut session = runtime.session;
                let mut status = runtime.status;
                let mut active_capture = runtime.active_capture;
                let mut active_on_frame = runtime.active_on_frame;
                let mut active_uplink = runtime.active_uplink;
                session.set(Some(next_session));
                status.set(MicrophoneStatus::Live);
                active_capture.set(capture);
                active_on_frame.set(Some(on_frame));
                active_uplink.set(uplink);
            }
            Err(error) => {
                if !operation_is_current(&runtime, operation_generation) {
                    return;
                }
                let next_status = status_from_error(error.clone());
                warn!(%error, status = ?next_status, "failed to start microphone capture");
                clear_failed_capture(&runtime, next_status);
            }
        }
    });
}

fn restart_capture(
    runtime: MicrophoneRuntime,
    operation_generation: u64,
    on_frame: MicrophoneFrameCallback,
    capture: ActiveCapture,
    uplink: Option<MicrophoneUplinkConfig>,
) {
    let previous_session = runtime.session.peek().clone();
    stop_session_immediately(previous_session.as_ref());
    prepare_capture_state(&runtime, on_frame.clone(), capture, uplink.clone());
    let config = capture_config(&runtime);

    spawn(async move {
        if let Some(previous_session) = previous_session
            && let Err(error) = previous_session.stop().await
        {
            warn!(%error, "failed to stop previous microphone capture before restart");
        }
        if !operation_is_current(&runtime, operation_generation) {
            return;
        }

        let callbacks = microphone_callbacks(
            on_frame.clone(),
            runtime.level,
            runtime.level_active,
            runtime.status,
            uplink.clone(),
        );
        match runtime.backend.start(config, callbacks).await {
            Ok(next_session) => {
                if !operation_is_current(&runtime, operation_generation) {
                    next_session.stop_immediately();
                    if let Err(error) = next_session.stop().await {
                        warn!(%error, "failed to stop stale microphone capture after restart");
                    }
                    return;
                }
                let mut session = runtime.session;
                let mut status = runtime.status;
                let mut active_capture = runtime.active_capture;
                let mut active_on_frame = runtime.active_on_frame;
                let mut active_uplink = runtime.active_uplink;
                session.set(Some(next_session));
                status.set(MicrophoneStatus::Live);
                active_capture.set(capture);
                active_on_frame.set(Some(on_frame));
                active_uplink.set(uplink);
            }
            Err(error) => {
                if !operation_is_current(&runtime, operation_generation) {
                    return;
                }
                let next_status = status_from_error(error.clone());
                warn!(%error, status = ?next_status, "failed to restart microphone capture");
                clear_failed_capture(&runtime, next_status);
            }
        }
    });
}

fn stop_capture(runtime: MicrophoneRuntime) {
    let active_session = runtime.session.peek().clone();
    stop_session_immediately(active_session.as_ref());
    let mut session = runtime.session;
    let mut status = runtime.status;
    let mut level = runtime.level;
    let mut level_active = runtime.level_active;
    let mut active_capture = runtime.active_capture;
    let mut active_on_frame = runtime.active_on_frame;
    let mut active_uplink = runtime.active_uplink;
    session.set(None);
    reset_level(&mut level, &mut level_active);
    status.set(MicrophoneStatus::Idle);
    active_capture.set(ActiveCapture::None);
    active_on_frame.set(None);
    active_uplink.set(None);

    let Some(active_session) = active_session else {
        return;
    };
    spawn(async move {
        if let Err(error) = active_session.stop().await {
            warn!(%error, "failed to stop microphone capture cleanly");
        }
    });
}

fn update_bitrate(runtime: MicrophoneRuntime, bitrate_bps: u32) {
    let Some(active_session) = runtime.session.peek().clone() else {
        return;
    };
    spawn(async move {
        if let Err(error) = active_session.set_bitrate_bps(bitrate_bps).await {
            warn!(%error, bitrate_bps, "failed to update microphone bitrate");
        }
    });
}

fn prepare_capture_state(
    runtime: &MicrophoneRuntime,
    on_frame: MicrophoneFrameCallback,
    capture: ActiveCapture,
    uplink: Option<MicrophoneUplinkConfig>,
) {
    let mut status = runtime.status;
    let mut level = runtime.level;
    let mut level_active = runtime.level_active;
    let mut active_capture = runtime.active_capture;
    let mut active_on_frame = runtime.active_on_frame;
    let mut active_uplink = runtime.active_uplink;
    status.set(MicrophoneStatus::Starting);
    active_capture.set(capture);
    active_on_frame.set(Some(on_frame));
    active_uplink.set(uplink);
    reset_level(&mut level, &mut level_active);
}

fn clear_failed_capture(runtime: &MicrophoneRuntime, next_status: MicrophoneStatus) {
    let mut session = runtime.session;
    let mut level = runtime.level;
    let mut level_active = runtime.level_active;
    let mut status = runtime.status;
    let mut active_capture = runtime.active_capture;
    let mut active_on_frame = runtime.active_on_frame;
    let mut active_uplink = runtime.active_uplink;
    session.set(None);
    reset_level(&mut level, &mut level_active);
    status.set(next_status);
    active_capture.set(ActiveCapture::None);
    active_on_frame.set(None);
    active_uplink.set(None);
}

fn capture_config(runtime: &MicrophoneRuntime) -> MicrophoneConfig {
    MicrophoneConfig {
        device_id: runtime.selected_input_device_id.peek().clone(),
        input_gain: gain_from_percent(*runtime.input_volume_percent.peek()),
        activation_mode: *runtime.activation_mode.peek(),
        vad_threshold: threshold_from_percent(*runtime.vad_threshold_percent.peek()),
        bitrate_bps: *runtime.target_bitrate_bps.peek(),
        ..MicrophoneConfig::default()
    }
}

fn operation_is_current(runtime: &MicrophoneRuntime, expected: u64) -> bool {
    let current = *runtime.generation.peek();
    if current == expected {
        return true;
    }
    debug!(
        expected,
        current, "discarded stale microphone runtime command"
    );
    false
}

pub(super) fn stop_session_immediately(session: Option<&Rc<dyn MicrophoneSession>>) {
    if let Some(session) = session {
        session.stop_immediately();
    }
}

#[derive(Debug, Clone, Copy)]
struct LevelEmissionState {
    timestamp_us: u64,
    active: bool,
}

pub(super) fn microphone_callbacks(
    on_frame: MicrophoneFrameCallback,
    level: Signal<MicrophoneLevel>,
    level_active: Signal<bool>,
    status: Signal<MicrophoneStatus>,
    uplink: Option<MicrophoneUplinkConfig>,
) -> super::backend::MicrophoneCallbacks {
    let (events, receiver) = mpsc::unbounded();
    spawn_microphone_callback_relay(receiver, on_frame, level, level_active, status);

    let frame_events = events.clone();
    let error_events = events.clone();
    super::backend::MicrophoneCallbacks {
        on_frame: Rc::new(move |frame| {
            let _ = frame_events.unbounded_send(MicrophoneCallbackEvent::Frame(frame));
        }),
        on_level: Rc::new(move |next_level| {
            let _ = events.unbounded_send(MicrophoneCallbackEvent::Level(next_level));
        }),
        on_error: Rc::new(move |error| {
            let _ = error_events.unbounded_send(MicrophoneCallbackEvent::Error(error));
        }),
        uplink,
    }
}

fn spawn_microphone_callback_relay(
    mut receiver: mpsc::UnboundedReceiver<MicrophoneCallbackEvent>,
    on_frame: MicrophoneFrameCallback,
    mut level: Signal<MicrophoneLevel>,
    mut level_active: Signal<bool>,
    mut status: Signal<MicrophoneStatus>,
) {
    spawn(async move {
        let emission = Rc::new(RefCell::new(None::<LevelEmissionState>));
        while let Some(event) = receiver.next().await {
            match event {
                MicrophoneCallbackEvent::Frame(frame) => on_frame(frame),
                MicrophoneCallbackEvent::Level(next_level) => {
                    if should_emit_level(&emission, next_level) {
                        if *level_active.peek() != next_level.active {
                            level_active.set(next_level.active);
                        }
                        level.set(next_level);
                    }
                }
                MicrophoneCallbackEvent::Error(error) => {
                    warn!(%error, "active microphone backend failed");
                    status.set(status_from_error(error));
                    reset_level(&mut level, &mut level_active);
                }
            }
        }
        debug!("microphone callback relay stopped");
    });
}

enum MicrophoneCallbackEvent {
    Frame(EncodedMicrophoneFrame),
    Level(MicrophoneLevel),
    Error(MicrophoneError),
}

pub(super) fn status_from_error(error: MicrophoneError) -> MicrophoneStatus {
    if error.is_permission_denied() {
        MicrophoneStatus::PermissionDenied
    } else {
        MicrophoneStatus::Error(error.to_string())
    }
}

pub(super) fn next_generation(generation: &mut Signal<u64>) -> u64 {
    let next_generation = generation.peek().saturating_add(1);
    generation.set(next_generation);
    next_generation
}

pub(super) fn reset_level(level: &mut Signal<MicrophoneLevel>, level_active: &mut Signal<bool>) {
    if *level_active.peek() {
        level_active.set(false);
    }
    level.set(default_level());
}

pub(super) fn default_level() -> MicrophoneLevel {
    MicrophoneLevel {
        rms: 0.0,
        active: false,
        threshold: MicrophoneConfig::default().vad_threshold,
        timestamp_us: 0,
    }
}

pub(super) fn gain_from_percent(volume_percent: u32) -> f32 {
    volume_percent.min(200) as f32 / 100.0
}

pub(super) fn threshold_from_percent(threshold_percent: u32) -> f32 {
    threshold_percent.min(100) as f32 / 1000.0
}

pub(super) fn persist_input_device(device_id: Option<&str>, label: Option<&str>) {
    match device_id {
        Some(device_id) if !device_id.is_empty() => {
            storage::save_input_device(device_id, label);
            info!(
                has_device = true,
                "persisted microphone input device preference"
            );
        }
        _ => {
            storage::clear_input_device_id();
            info!(
                has_device = false,
                "cleared microphone input device preference"
            );
        }
    }
}

fn should_emit_level(
    emission: &Rc<RefCell<Option<LevelEmissionState>>>,
    next_level: MicrophoneLevel,
) -> bool {
    let mut emission = emission.borrow_mut();
    let Some(previous) = *emission else {
        *emission = Some(LevelEmissionState {
            timestamp_us: next_level.timestamp_us,
            active: next_level.active,
        });
        return true;
    };

    let active_changed = previous.active != next_level.active;
    let interval_elapsed = next_level.timestamp_us > previous.timestamp_us
        && next_level
            .timestamp_us
            .saturating_sub(previous.timestamp_us)
            >= MICROPHONE_LEVEL_UPDATE_INTERVAL_US;
    if !active_changed && !interval_elapsed {
        return false;
    }

    *emission = Some(LevelEmissionState {
        timestamp_us: next_level.timestamp_us,
        active: next_level.active,
    });
    true
}

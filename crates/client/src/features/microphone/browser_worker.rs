//! Browser bridge между AudioWorklet и Dedicated Worker uplink микрофона.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::{info, warn};
use futures_channel::oneshot;
use futures_util::future::{Either, select};
use gloo_timers::future::TimeoutFuture;
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{ErrorEvent, MessageEvent, MessagePort, Worker, WorkerOptions, WorkerType};

use super::backend::{
    MicrophoneActivationMode, MicrophoneCallbacks, MicrophoneConfig, MicrophoneError,
    MicrophoneLevel, MicrophoneLevelCallback, MicrophoneUplinkConfig,
};
use super::browser_errors::js_error_message;

const MICROPHONE_UPLINK_WORKER_URL: &str = "/audio/microphone-uplink-worker.js?v=4";
const MICROPHONE_WORKER_WASM_BINDGEN_URL: &str = "/workers/microphone/microphone_worker.js?v=3";
const MICROPHONE_WORKER_WASM_URL: &str = "/workers/microphone/microphone_worker_bg.wasm?v=3";
const WORKER_START_TIMEOUT_MS: u32 = 10_000;

type WorkerReadySender = Rc<RefCell<Option<oneshot::Sender<Result<(), String>>>>>;

pub(super) struct BrowserWorkerUplink {
    worker: Worker,
    _message_closure: Closure<dyn FnMut(MessageEvent)>,
    _error_closure: Closure<dyn FnMut(ErrorEvent)>,
}

impl BrowserWorkerUplink {
    pub(super) fn stop(&self) {
        let stop = Object::new();
        if set_property(&stop, "kind", JsValue::from_str("stop")).is_ok() {
            let _ = self.worker.post_message(stop.as_ref());
        }
        self.worker.terminate();
    }

    pub(super) fn set_bitrate_bps(&self, bitrate_bps: u32) {
        let message = Object::new();
        if set_property(&message, "kind", JsValue::from_str("set-bitrate")).is_ok()
            && set_property(
                &message,
                "bitrateBps",
                JsValue::from_f64(f64::from(bitrate_bps)),
            )
            .is_ok()
        {
            let _ = self.worker.post_message(message.as_ref());
        }
    }
}

pub(super) async fn start_worker_uplink(
    port: MessagePort,
    callbacks: &MicrophoneCallbacks,
    config: &MicrophoneConfig,
    uplink: MicrophoneUplinkConfig,
    sample_rate_hz: u32,
) -> Result<BrowserWorkerUplink, MicrophoneError> {
    let worker = module_worker(MICROPHONE_UPLINK_WORKER_URL)?;
    let (ready_sender, ready_receiver) = oneshot::channel::<Result<(), String>>();
    let ready_sender = Rc::new(RefCell::new(Some(ready_sender)));
    let message_ready = ready_sender.clone();
    let on_level = callbacks.on_level.clone();
    let on_error = callbacks.on_error.clone();
    let message_closure = Closure::wrap(Box::new(move |event: MessageEvent| {
        handle_worker_message(event.data(), &on_level, &on_error, &message_ready);
    }) as Box<dyn FnMut(MessageEvent)>);
    worker.set_onmessage(Some(message_closure.as_ref().unchecked_ref()));

    let error_ready = ready_sender.clone();
    let worker_on_error = callbacks.on_error.clone();
    let error_closure = Closure::wrap(Box::new(move |event: ErrorEvent| {
        let message = event.message();
        if let Some(sender) = error_ready.borrow_mut().take() {
            let _ = sender.send(Err(message.clone()));
        }
        worker_on_error(MicrophoneError::new(message.clone()));
        warn!(
            message = %message,
            filename = %event.filename(),
            line = event.lineno(),
            "microphone uplink worker failed"
        );
    }) as Box<dyn FnMut(ErrorEvent)>);
    worker.set_onerror(Some(error_closure.as_ref().unchecked_ref()));

    let start = start_message(&port, config, uplink, sample_rate_hz)?;
    let transfer = Array::new();
    transfer.push(port.as_ref());
    worker
        .post_message_with_transfer(start.as_ref(), &transfer)
        .map_err(microphone_error)?;

    let ready = select(
        Box::pin(ready_receiver),
        Box::pin(TimeoutFuture::new(WORKER_START_TIMEOUT_MS)),
    )
    .await;
    match ready {
        Either::Left((Ok(Ok(())), _)) => {}
        Either::Left((Ok(Err(message)), _)) => {
            worker.terminate();
            return Err(MicrophoneError::new(message));
        }
        Either::Left((Err(_), _)) => {
            worker.terminate();
            return Err(MicrophoneError::new(
                "Dedicated Worker микрофона завершился до готовности.",
            ));
        }
        Either::Right(((), _)) => {
            worker.terminate();
            return Err(MicrophoneError::new(
                "Dedicated Worker микрофона не запустился вовремя.",
            ));
        }
    }

    info!(sample_rate_hz, "microphone uplink worker ready");
    Ok(BrowserWorkerUplink {
        worker,
        _message_closure: message_closure,
        _error_closure: error_closure,
    })
}

fn module_worker(url: &str) -> Result<Worker, MicrophoneError> {
    let options = WorkerOptions::new();
    options.set_type(WorkerType::Module);
    Worker::new_with_options(url, &options).map_err(microphone_error)
}

fn start_message(
    port: &MessagePort,
    config: &MicrophoneConfig,
    uplink: MicrophoneUplinkConfig,
    sample_rate_hz: u32,
) -> Result<Object, MicrophoneError> {
    let message = Object::new();
    set_property(&message, "kind", JsValue::from_str("start"))?;
    set_property(&message, "workletPort", JsValue::from(port.clone()))?;
    set_property(
        &message,
        "wasmBindgenUrl",
        JsValue::from_str(MICROPHONE_WORKER_WASM_BINDGEN_URL),
    )?;
    set_property(
        &message,
        "wasmUrl",
        JsValue::from_str(MICROPHONE_WORKER_WASM_URL),
    )?;
    set_property(
        &message,
        "realtimeUrl",
        JsValue::from_str(&uplink.realtime_url),
    )?;
    set_property(
        &message,
        "realtimeWebsocketUrl",
        JsValue::from_str(&uplink.realtime_websocket_url),
    )?;
    set_property(
        &message,
        "realtimeCertSha256",
        uplink
            .realtime_cert_sha256
            .as_deref()
            .map(JsValue::from_str)
            .unwrap_or(JsValue::NULL),
    )?;
    set_property(
        &message,
        "accessToken",
        JsValue::from_str(&uplink.access_token),
    )?;
    set_property(&message, "uplinkGrant", JsValue::from_str(&uplink.grant))?;
    set_property(&message, "roomId", JsValue::from_str(&uplink.room_id))?;
    set_property(
        &message,
        "sampleRateHz",
        JsValue::from_f64(f64::from(sample_rate_hz)),
    )?;
    set_property(
        &message,
        "channels",
        JsValue::from_f64(f64::from(config.channels)),
    )?;
    set_property(
        &message,
        "bitrateBps",
        JsValue::from_f64(f64::from(config.bitrate_bps)),
    )?;
    set_property(
        &message,
        "activationMode",
        JsValue::from_str(match config.activation_mode {
            MicrophoneActivationMode::AlwaysActive => "always_active",
            MicrophoneActivationMode::VoiceActivated => "voice_activated",
        }),
    )?;
    set_property(
        &message,
        "vadThreshold",
        JsValue::from_f64(f64::from(config.vad_threshold)),
    )?;
    set_property(
        &message,
        "vadActivationDelayUs",
        JsValue::from_f64(f64::from(config.vad_activation_delay_us)),
    )?;
    set_property(
        &message,
        "vadReleaseDelayUs",
        JsValue::from_f64(f64::from(config.vad_release_delay_us)),
    )?;
    set_property(
        &message,
        "inputGain",
        JsValue::from_f64(f64::from(config.input_gain)),
    )?;
    Ok(message)
}

fn handle_worker_message(
    data: JsValue,
    on_level: &MicrophoneLevelCallback,
    on_error: &super::backend::MicrophoneErrorCallback,
    ready_sender: &WorkerReadySender,
) {
    let kind = string_property(&data, "kind").unwrap_or_default();
    match kind.as_str() {
        "ready" => {
            if let Some(sender) = ready_sender.borrow_mut().take() {
                let _ = sender.send(Ok(()));
            }
        }
        "level" => on_level(MicrophoneLevel {
            rms: number_property(&data, "rms").unwrap_or_default() as f32,
            active: bool_property(&data, "active").unwrap_or(false),
            threshold: number_property(&data, "threshold").unwrap_or_default() as f32,
            timestamp_us: number_property(&data, "timestampUs")
                .unwrap_or_default()
                .max(0.0) as u64,
        }),
        "warning" => warn!(
            message = %string_property(&data, "message").unwrap_or_else(|| "worker warning".to_owned()),
            detail = ?string_property(&data, "detail"),
            dropped_pcm = ?number_property(&data, "droppedPcm"),
            dropped_encoded = ?number_property(&data, "droppedEncoded"),
            "microphone uplink worker warning"
        ),
        "error" => {
            let message = string_property(&data, "message")
                .unwrap_or_else(|| "microphone worker error".to_owned());
            if let Some(sender) = ready_sender.borrow_mut().take() {
                let _ = sender.send(Err(message.clone()));
            }
            on_error(MicrophoneError::new(message.clone()));
            warn!(message = %message, "microphone uplink worker error");
        }
        "status" => info!(
            message = %string_property(&data, "message").unwrap_or_else(|| "worker status".to_owned()),
            transport = ?string_property(&data, "transport"),
            "microphone uplink worker status"
        ),
        _ => warn!(kind, "microphone uplink worker sent unknown message"),
    }
}

fn string_property(value: &JsValue, name: &str) -> Option<String> {
    Reflect::get(value, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_string())
}

fn number_property(value: &JsValue, name: &str) -> Option<f64> {
    Reflect::get(value, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_f64())
}

fn bool_property(value: &JsValue, name: &str) -> Option<bool> {
    Reflect::get(value, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_bool())
}

fn set_property(object: &Object, name: &str, value: JsValue) -> Result<(), MicrophoneError> {
    Reflect::set(object, &JsValue::from_str(name), &value)
        .map(|_| ())
        .map_err(microphone_error)
}

fn microphone_error(error: JsValue) -> MicrophoneError {
    MicrophoneError::new(js_error_message(error))
}

//! wasm API worker микрофона для обработки PCM и кодирования protocol frames.

use cheenhub_contracts::media::{MediaCodec, MediaDatagram, MediaDatagramKind};
use cheenhub_contracts::realtime::{
    Authenticate, BindMicrophoneUplink, ControlKind, RealtimeEnvelope, RealtimeKind,
    RealtimeModule, VoiceChatKind,
};
use js_sys::{Float32Array, Object, Reflect, Uint8Array};
use uuid::Uuid;
use wasm_bindgen::prelude::*;

use crate::microphone_core::{
    CoreActivationMode, VoiceActivationConfig, VoiceActivityDetector, apply_input_gain,
    duration_us, rms_level,
};

const MICROPHONE_WORKER_ABI_VERSION: u32 = 3;

/// Возвращает версию ABI worker микрофона.
#[wasm_bindgen]
pub fn microphone_worker_abi_version() -> u32 {
    MICROPHONE_WORKER_ABI_VERSION
}

/// Создает length-prefixed auth frame для reliable WebTransport stream.
#[wasm_bindgen]
pub fn microphone_worker_authenticate_webtransport_frame(
    access_token: &str,
) -> Result<Uint8Array, JsValue> {
    length_prefixed_envelope(authenticate_envelope(access_token)?)
}

/// Создает JSON auth envelope для WebSocket fallback.
#[wasm_bindgen]
pub fn microphone_worker_authenticate_websocket_message(
    access_token: &str,
) -> Result<String, JsValue> {
    encode_envelope_json(&authenticate_envelope(access_token)?)
}

/// Создает length-prefixed bind frame для WebTransport control stream.
#[wasm_bindgen]
pub fn microphone_worker_bind_uplink_webtransport_frame(
    grant: &str,
) -> Result<Uint8Array, JsValue> {
    length_prefixed_envelope(bind_uplink_envelope(grant)?)
}

/// Создает JSON bind envelope для WebSocket fallback.
#[wasm_bindgen]
pub fn microphone_worker_bind_uplink_websocket_message(grant: &str) -> Result<String, JsValue> {
    encode_envelope_json(&bind_uplink_envelope(grant)?)
}

/// Stateful processor PCM chunks и encoded voice datagrams.
#[wasm_bindgen]
pub struct MicrophoneWorkerProcessor {
    detector: VoiceActivityDetector,
    sample_rate_hz: u32,
    input_gain: f32,
    room_id: Uuid,
    sequence: u64,
}

#[wasm_bindgen]
impl MicrophoneWorkerProcessor {
    /// Создает processor для одной активной комнаты.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sample_rate_hz: u32,
        channels: u8,
        activation_mode: &str,
        vad_threshold: f32,
        vad_activation_delay_us: u32,
        vad_release_delay_us: u32,
        input_gain: f32,
        room_id: &str,
    ) -> Result<MicrophoneWorkerProcessor, JsValue> {
        if channels != 1 {
            return Err(js_error("microphone worker supports only mono input"));
        }
        let room_id = Uuid::parse_str(room_id)
            .map_err(|error| js_error(format!("invalid voice room id: {error}")))?;
        let mode = match activation_mode {
            "always_active" => CoreActivationMode::AlwaysActive,
            "voice_activated" => CoreActivationMode::VoiceActivated,
            value => {
                return Err(js_error(format!(
                    "unsupported microphone activation mode: {value}"
                )));
            }
        };
        let detector = VoiceActivityDetector::new(VoiceActivationConfig {
            mode,
            threshold: vad_threshold,
            activation_delay_us: vad_activation_delay_us,
            release_delay_us: vad_release_delay_us,
        });
        Ok(Self {
            detector,
            sample_rate_hz,
            input_gain,
            room_id,
            sequence: 0,
        })
    }

    /// Обрабатывает PCM chunk и возвращает level/VAD результат.
    pub fn process_pcm(
        &mut self,
        samples: &Float32Array,
        timestamp_us: f64,
    ) -> Result<JsValue, JsValue> {
        let mut samples = samples.to_vec();
        apply_input_gain(&mut samples, self.input_gain);
        let duration_us = duration_us(samples.len(), self.sample_rate_hz);
        let rms = rms_level(&samples);
        let active = self.detector.update(rms, duration_us);
        let output = Object::new();
        set_property(&output, "rms", JsValue::from_f64(f64::from(rms)))?;
        set_property(&output, "active", JsValue::from_bool(active))?;
        set_property(
            &output,
            "threshold",
            JsValue::from_f64(f64::from(self.detector.config().threshold)),
        )?;
        set_property(
            &output,
            "timestampUs",
            JsValue::from_f64(timestamp_us.max(0.0)),
        )?;
        set_property(
            &output,
            "durationUs",
            JsValue::from_f64(f64::from(duration_us)),
        )?;
        if active {
            set_property(
                &output,
                "samples",
                JsValue::from(Float32Array::from(samples.as_slice())),
            )?;
        } else {
            set_property(&output, "samples", JsValue::NULL)?;
        }
        Ok(output.into())
    }

    /// Кодирует voice datagram для WebTransport или WebSocket fallback.
    pub fn voice_datagram(
        &mut self,
        payload: &Uint8Array,
        timestamp_us: f64,
        duration_us: f64,
    ) -> Result<Uint8Array, JsValue> {
        let datagram = MediaDatagram {
            kind: MediaDatagramKind::VoiceFrame,
            codec: MediaCodec::Opus,
            flags: 0,
            sequence: self.sequence,
            timestamp_us: timestamp_us.max(0.0) as u64,
            duration_us: duration_us.max(0.0).min(f64::from(u32::MAX)) as u32,
            room_id: self.room_id,
            sender_user_id: Uuid::nil(),
            payload: payload.to_vec(),
        };
        self.sequence = self.sequence.saturating_add(1);
        let bytes = datagram
            .encode()
            .map_err(|error| js_error(format!("failed to encode voice datagram: {error}")))?;
        Ok(Uint8Array::from(bytes.as_slice()))
    }
}

fn authenticate_envelope(access_token: &str) -> Result<RealtimeEnvelope, JsValue> {
    RealtimeEnvelope::new(
        RealtimeModule::Control,
        RealtimeKind::Control(ControlKind::Authenticate),
        Some(Uuid::new_v4()),
        Authenticate {
            access_token: access_token.to_owned(),
        },
    )
    .map_err(|error| js_error(format!("failed to build auth envelope: {error}")))
}

fn bind_uplink_envelope(grant: &str) -> Result<RealtimeEnvelope, JsValue> {
    RealtimeEnvelope::new(
        RealtimeModule::VoiceChat,
        RealtimeKind::VoiceChat(VoiceChatKind::BindMicrophoneUplink),
        Some(Uuid::new_v4()),
        BindMicrophoneUplink {
            grant: grant.to_owned(),
        },
    )
    .map_err(|error| {
        js_error(format!(
            "failed to build microphone uplink bind envelope: {error}"
        ))
    })
}

fn length_prefixed_envelope(envelope: RealtimeEnvelope) -> Result<Uint8Array, JsValue> {
    let payload = serde_json::to_vec(&envelope)
        .map_err(|error| js_error(format!("failed to encode realtime envelope: {error}")))?;
    let length =
        u32::try_from(payload.len()).map_err(|_| js_error("realtime frame is too large"))?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(Uint8Array::from(frame.as_slice()))
}

fn encode_envelope_json(envelope: &RealtimeEnvelope) -> Result<String, JsValue> {
    serde_json::to_string(envelope)
        .map_err(|error| js_error(format!("failed to encode realtime envelope: {error}")))
}

fn set_property(object: &Object, name: &str, value: JsValue) -> Result<(), JsValue> {
    Reflect::set(object, &JsValue::from_str(name), &value).map(|_| ())
}

fn js_error(message: impl AsRef<str>) -> JsValue {
    JsValue::from_str(message.as_ref())
}

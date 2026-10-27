//! Microphone backend contracts.

use std::fmt;
use std::rc::Rc;

use futures_util::future::LocalBoxFuture;

/// Callback invoked for every encoded microphone frame.
pub(crate) type MicrophoneFrameCallback = Rc<dyn Fn(EncodedMicrophoneFrame)>;

/// Callback invoked for every measured microphone level sample.
pub(crate) type MicrophoneLevelCallback = Rc<dyn Fn(MicrophoneLevel)>;

/// Callback invoked when an active backend fails after startup.
pub(crate) type MicrophoneErrorCallback = Rc<dyn Fn(MicrophoneError)>;
/// Callback for recoverable processing degradation while capture remains active.
pub(crate) type MicrophoneWarningCallback = Rc<dyn Fn(&'static str)>;

/// Настройки отдельного low-latency uplink вне UI runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MicrophoneUplinkConfig {
    /// Access JWT для аутентификации отдельной realtime-сессии.
    pub(crate) access_token: String,
    /// Одноразовый grant для привязки uplink к текущему присутствию.
    pub(crate) grant: String,
    /// Идентификатор целевой комнаты.
    pub(crate) room_id: String,
    /// URL основного WebTransport endpoint.
    pub(crate) realtime_url: String,
    /// URL WebSocket fallback.
    pub(crate) realtime_websocket_url: String,
    /// Необязательный SHA-256 fingerprint dev-сертификата.
    pub(crate) realtime_cert_sha256: Option<String>,
}

/// Encoded microphone codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MicrophoneCodec {
    /// Opus audio.
    Opus,
}

/// Microphone audio activation behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MicrophoneActivationMode {
    /// Encode every captured frame while the microphone is live.
    #[allow(dead_code)]
    AlwaysActive,
    /// Encode frames only while voice activation is open.
    VoiceActivated,
    /// Передаёт звук только при удержании глобальной клавиши на поддерживаемой платформе.
    PushToTalk,
}

/// Живые параметры gate микрофона; изменение не требует новой capture/uplink-сессии.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ActivationConfig {
    /// Режим передачи; переходы к Push-to-talk требуют отдельного capture setup.
    pub(crate) mode: MicrophoneActivationMode,
    /// Нормализованный RMS-порог, используемый без RNNoise.
    pub(crate) threshold: f32,
    /// Порог вероятности речи RNNoise, независимый от RMS.
    pub(crate) neural_threshold: f32,
    /// Длительность сигнала для открытия gate, в микросекундах.
    pub(crate) activation_delay_us: u32,
    /// Удержание gate после окончания речи, в микросекундах.
    pub(crate) release_delay_us: u32,
}

/// Microphone capture and encoding configuration.
#[derive(Debug, Clone)]
pub(crate) struct MicrophoneConfig {
    /// Preferred encoded codec.
    pub(crate) codec: MicrophoneCodec,
    /// Target sample rate in hertz.
    pub(crate) sample_rate_hz: u32,
    /// Target channel count.
    pub(crate) channels: u8,
    /// Target encoder bitrate in bits per second.
    pub(crate) bitrate_bps: u32,
    /// Audio activation mode used before encoding.
    pub(crate) activation_mode: MicrophoneActivationMode,
    /// Живые параметры gate текущей сессии; `None` сохраняет стартовую конфигурацию.
    pub(crate) activation_live: Option<std::sync::Arc<std::sync::Mutex<ActivationConfig>>>,
    /// Проверенная привязка глобального Push-to-talk.
    pub(crate) push_to_talk_key: super::push_to_talk::PushToTalkKey,
    /// Поколение назначения кнопки; используется для паузы capture и отзыва queued frames.
    pub(crate) recording_generation: std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// RMS level threshold that opens voice activation.
    pub(crate) vad_threshold: f32,
    /// Порог вероятности речи RNNoise в диапазоне 0–1.
    pub(crate) neural_vad_threshold: f32,
    /// Time the level must stay above threshold before activation opens.
    pub(crate) vad_activation_delay_us: u32,
    /// Time activation remains open after the level falls below threshold.
    pub(crate) vad_release_delay_us: u32,
    /// Идентификатор устройства ввода для platform backend; `None` использует системное устройство по умолчанию.
    pub(crate) device_id: Option<String>,
    /// Linear input gain applied before voice activation and encoding.
    pub(crate) input_gain: f32,
    /// Whether Windows worker DSP is enabled for this capture.
    pub(crate) denoise_enabled: bool,
    /// Включает встроенный denoiser браузера вместо RNNoise.
    pub(crate) browser_denoise_enabled: bool,
    /// Worker-owned runtime switch sampled at frame boundaries. A failed DSP clears this without
    /// changing the persisted user preference.
    pub(crate) denoise_enabled_live: Option<std::sync::Arc<super::denoise::RuntimeSwitch>>,
}

impl Default for MicrophoneConfig {
    fn default() -> Self {
        Self {
            codec: MicrophoneCodec::Opus,
            sample_rate_hz: 48_000,
            channels: 1,
            bitrate_bps: cheenhub_contracts::media::VOICE_AUDIO_BITRATE_BPS,
            activation_mode: MicrophoneActivationMode::VoiceActivated,
            activation_live: None,
            push_to_talk_key: super::push_to_talk::PushToTalkKey::default(),
            recording_generation: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            vad_threshold: 0.02,
            neural_vad_threshold: 0.5,
            vad_activation_delay_us: 60_000,
            vad_release_delay_us: 250_000,
            device_id: None,
            input_gain: 1.0,
            denoise_enabled: false,
            browser_denoise_enabled: false,
            denoise_enabled_live: None,
        }
    }
}

/// Microphone callbacks supplied by the owning feature.
#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct MicrophoneCallbacks {
    /// Encoded frame callback.
    pub(crate) on_frame: MicrophoneFrameCallback,
    /// Measured input level callback.
    pub(crate) on_level: MicrophoneLevelCallback,
    /// Ошибка уже запущенной capture/uplink-сессии.
    pub(crate) on_error: MicrophoneErrorCallback,
    /// Recoverable DSP warnings that must not change capture status.
    pub(crate) on_warning: MicrophoneWarningCallback,
    /// Отдельный uplink, который browser backend должен запустить вместо on_frame path.
    pub(crate) uplink: Option<MicrophoneUplinkConfig>,
}

/// One encoded microphone frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EncodedMicrophoneFrame {
    /// UUID текущего захвата; сохраняется при live-изменении gain, VAD и bitrate.
    pub(crate) track_id: uuid::Uuid,
    /// Разрешение исходного capture; `None` для кадров без отзываемого gate.
    pub(crate) permission: Option<super::frame_permission::FramePermission>,
    /// Sender-local frame sequence.
    pub(crate) sequence: u64,
    /// Frame timestamp in microseconds.
    pub(crate) timestamp_us: u64,
    /// Frame duration in microseconds.
    pub(crate) duration_us: u32,
    /// Encoded codec.
    pub(crate) codec: MicrophoneCodec,
    /// Sample rate in hertz.
    pub(crate) sample_rate_hz: u32,
    /// Channel count.
    pub(crate) channels: u8,
    /// Raw encoded frame bytes.
    pub(crate) bytes: Vec<u8>,
}

impl EncodedMicrophoneFrame {
    /// Проверяет допустимость отложенной передачи без раскрытия состояния платформы.
    pub(crate) fn can_send(&self) -> bool {
        self.permission
            .as_ref()
            .is_none_or(|permission| permission.allowed())
    }
}

/// Current microphone input level for visualization and threshold tuning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MicrophoneLevel {
    /// Input RMS amplitude normalized to roughly `0.0..=1.0`.
    pub(crate) rms: f32,
    /// Whether the activation gate is currently open.
    pub(crate) active: bool,
    /// Источник порога: RMS или вероятность речи RNNoise.
    pub(crate) source: &'static str,
    /// Active threshold used for this measurement.
    pub(crate) threshold: f32,
    /// Capture timestamp in microseconds when provided by the backend.
    pub(crate) timestamp_us: u64,
}

/// Current microphone capture status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MicrophoneStatus {
    /// Capture is stopped.
    Idle,
    /// Browser or backend permission/capture startup is in flight.
    Starting,
    /// Capture and encoding are active.
    Live,
    /// Browser or OS denied microphone permission.
    PermissionDenied,
    /// Last microphone operation failed.
    Error(String),
}

/// Active microphone session.
pub(crate) trait MicrophoneSession {
    /// Немедленно прекращает захват, не ожидая асинхронного освобождения ресурсов.
    fn stop_immediately(&self);

    /// Stops capture and releases backend resources.
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), MicrophoneError>>;

    /// Updates target bitrate when the backend supports live reconfiguration.
    #[allow(dead_code)]
    fn set_bitrate_bps(
        &self,
        bitrate_bps: u32,
    ) -> LocalBoxFuture<'static, Result<(), MicrophoneError>>;

    /// Обновляет linear input gain без перезапуска активного захвата.
    fn set_input_gain(&self, input_gain: f32) -> Result<(), MicrophoneError>;

    /// Обновляет gate отдельного worker; main/native runtime читает общий snapshot.
    fn set_activation_config(&self, _config: ActivationConfig) -> Result<(), MicrophoneError> {
        Ok(())
    }

    /// Обновляет browser capture и worker без перезапуска; native DSP читает atomic.
    fn set_denoise_mode(
        &self,
        mode: super::denoise::DenoiseMode,
        runtime: std::sync::Arc<super::denoise::RuntimeSwitch>,
    ) -> LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        Box::pin(async move {
            runtime.store(
                mode == super::denoise::DenoiseMode::Rnnoise,
                std::sync::atomic::Ordering::Relaxed,
            );
            Ok(())
        })
    }
}

/// Microphone capture backend.
pub(crate) trait MicrophoneBackend {
    /// Starts capture and calls `on_frame` for every encoded frame.
    fn start(
        &self,
        config: MicrophoneConfig,
        callbacks: MicrophoneCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn MicrophoneSession>, MicrophoneError>>;
}

/// Microphone backend error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MicrophoneError {
    message: String,
    kind: MicrophoneErrorKind,
}

impl MicrophoneError {
    /// Builds a microphone error from a user-facing message.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: MicrophoneErrorKind::Unavailable,
        }
    }

    /// Builds a microphone permission-denied error.
    #[allow(dead_code)]
    pub(crate) fn permission_denied(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: MicrophoneErrorKind::PermissionDenied,
        }
    }

    /// Отмечает ошибку, после которой platform-сессию необходимо отозвать.
    pub(crate) fn invalidated(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: MicrophoneErrorKind::Invalidated,
        }
    }

    /// Указывает, нельзя ли продолжать работу этой capture-сессии.
    pub(crate) fn is_invalidated(&self) -> bool {
        self.kind == MicrophoneErrorKind::Invalidated
    }

    /// Returns whether the user or browser denied microphone access.
    pub(crate) fn is_permission_denied(&self) -> bool {
        self.kind == MicrophoneErrorKind::PermissionDenied
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MicrophoneErrorKind {
    PermissionDenied,
    Unavailable,
    Invalidated,
}

impl fmt::Display for MicrophoneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for MicrophoneError {}

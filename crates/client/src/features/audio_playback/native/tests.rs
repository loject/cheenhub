//! Тестовые фабрики native-воспроизведения.

use super::*;

impl AudioPlaybackHandle {
    /// Создаёт нейтральный контекст воспроизведения без запуска аудиоустройства для unit-тестов.
    pub(crate) fn for_test() -> Self {
        Self {
            muted: Signal::new(false),
            selected_output_device_id: Signal::new(None),
            selected_output_device_label: Signal::new(None),
            output_volume_percent: Signal::new(100),
            jitter_buffer_us: Signal::new(storage::DEFAULT_JITTER_BUFFER_US),
            inner: Rc::new(RefCell::new(AudioPlaybackInner {
                tracks: crate::features::audio_playback::tracks::PlaybackTracks::default(),
                muted: false,
                engine: None,
                decoders: HashMap::new(),
                time_compressors: HashMap::new(),
                playback_pauses: HashMap::new(),
                jitter_buffers: HashMap::new(),
                jitter_drainers: HashMap::new(),
                next_jitter_drainer_generation: 0,
                jitter_buffer_us: storage::DEFAULT_JITTER_BUFFER_US,
                jitter_warning_at_ms: HashMap::new(),
                decoder_warning_at_ms: HashMap::new(),
                user_volumes: HashMap::new(),
                output_gain: 1.0,
            })),
        }
    }
}

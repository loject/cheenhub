//! Output stream Android на базе Oboe с атрибутами голосовой связи.

use dioxus::prelude::{debug, info, warn};
use oboe::{
    AudioOutputCallback, AudioOutputStreamSafe, AudioStream, AudioStreamAsync, AudioStreamBase,
    AudioStreamBuilder, ContentType, DataCallbackResult, Error, Output, PerformanceMode,
    SharingMode, Stereo, Usage,
};

use super::cpal_playback::mixer::{MixerHandle, NativeOutputMixer, new_mixer};

struct AndroidOutputCallback {
    output: NativeOutputMixer,
}

impl AudioOutputCallback for AndroidOutputCallback {
    type FrameType = (f32, Stereo);

    fn on_audio_ready(
        &mut self,
        _audio_stream: &mut dyn AudioOutputStreamSafe,
        audio_data: &mut [(f32, f32)],
    ) -> DataCallbackResult {
        fill_oboe_stereo(&mut self.output, audio_data);
        DataCallbackResult::Continue
    }

    fn on_error_after_close(
        &mut self,
        _audio_stream: &mut dyn AudioOutputStreamSafe,
        error: Error,
    ) {
        warn!(%error, "Android Oboe voice playback stream closed after an error");
    }
}

/// Активный Android output stream и связанный с ним микшер.
pub(super) struct NativePlaybackEngine {
    _stream: AudioStreamAsync<Output, AndroidOutputCallback>,
    pub(super) mixer: MixerHandle,
    device_id: Option<String>,
}

impl NativePlaybackEngine {
    /// Возвращает идентификатор выбранного устройства вывода.
    pub(super) fn device_id(&self) -> Option<String> {
        self.device_id.clone()
    }
}

impl Drop for NativePlaybackEngine {
    fn drop(&mut self) {
        debug!("Android Oboe voice playback stream dropped");
    }
}

/// Создаёт Android output stream с системным профилем голосовой связи.
pub(super) fn create_engine(
    device_id: Option<String>,
    output_gain: f32,
    sample_rate_hz: u32,
) -> Result<NativePlaybackEngine, String> {
    if device_id.as_deref().is_some_and(|id| !id.is_empty()) {
        warn!(
            selected_device = device_id.as_deref().unwrap_or_default(),
            "Android ignores stored native output device; route is managed by AudioManager"
        );
    }

    let mixer = new_mixer(output_gain);
    let callback = AndroidOutputCallback {
        output: NativeOutputMixer::new(sample_rate_hz, sample_rate_hz, mixer.clone()),
    };
    let mut stream = AudioStreamBuilder::default()
        .set_performance_mode(PerformanceMode::LowLatency)
        .set_sharing_mode(SharingMode::Shared)
        .set_usage(Usage::VoiceCommunication)
        .set_content_type(ContentType::Speech)
        .set_sample_rate(sample_rate_hz as i32)
        .set_stereo()
        .set_f32()
        .set_callback(callback)
        .open_stream()
        .map_err(oboe_error)?;
    stream.start().map_err(oboe_error)?;

    info!(
        sample_rate_hz = stream.get_sample_rate(),
        channels = ?stream.get_channel_count(),
        usage = ?stream.get_usage(),
        content_type = ?stream.get_content_type(),
        "Android Oboe voice playback started"
    );
    Ok(NativePlaybackEngine {
        _stream: stream,
        mixer,
        device_id: None,
    })
}

fn oboe_error(error: oboe::Error) -> String {
    format!("Не удалось запустить Android Oboe voice output: {error}")
}

fn fill_oboe_stereo(output: &mut NativeOutputMixer, audio_data: &mut [(f32, f32)]) {
    output.render_frames(audio_data.len(), |frame_index, sample| {
        audio_data[frame_index] = (sample, sample);
    });
}

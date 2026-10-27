//! Browser-граф системного звука: stereo PCM, Opus и ограниченная очередь отправки.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use dioxus::prelude::{debug, warn};
use futures_channel::mpsc;
use js_sys::{Array, Float32Array, Object, Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    AudioContext, AudioContextOptions, AudioWorkletNode, AudioWorkletNodeOptions, GainNode,
    MediaStream, MediaStreamAudioSourceNode, MediaStreamTrack, MessageEvent,
};
use web_time::Instant;

use super::super::backend::{
    EncodedScreenAudioFrame, ScreenShareAudioState, ScreenShareCallbacks, ScreenShareError,
};
use super::super::browser_capture::{own_audio_restricted, whole_screen_selected};
use super::super::browser_errors::js_error_message;
use super::audio_bindings::{AudioData, AudioEncoder, EncodedAudioChunk};

/// Управляет аудиографом одной демонстрации; stop освобождает его синхронно.
pub(super) struct BrowserScreenAudio {
    graph: Rc<AudioGraph>,
}

struct AudioGraph {
    context: AudioContext,
    track: MediaStreamTrack,
    video: MediaStreamTrack,
    source: MediaStreamAudioSourceNode,
    node: AudioWorkletNode,
    gain: GainNode,
    encoder: AudioEncoder,
    closed: Rc<Cell<bool>>,
    receiver: RefCell<Option<mpsc::Receiver<EncodedScreenAudioFrame>>>,
    callbacks: ScreenShareCallbacks,
    timestamp_base_us: f64,
    _pcm: Closure<dyn FnMut(MessageEvent)>,
    _output: Closure<dyn FnMut(EncodedAudioChunk)>,
    _error: Closure<dyn FnMut(JsValue)>,
    _ended: Closure<dyn FnMut(web_sys::Event)>,
}

struct PendingAudio {
    context: AudioContext,
    track: MediaStreamTrack,
    encoder: Option<AudioEncoder>,
    transferred: bool,
}

impl Drop for PendingAudio {
    fn drop(&mut self) {
        if self.transferred {
            return;
        }
        self.track.stop();
        if let Some(encoder) = &self.encoder {
            let _ = encoder.close();
        }
        let _ = self.context.close();
    }
}

impl BrowserScreenAudio {
    /// Создаёт stereo Opus-граф и привязывает время PCM к первому видеокадру.
    ///
    /// # Errors
    /// Возвращает ошибку AudioContext, AudioWorklet или WebCodecs; незавершённый
    /// граф и его аудиодорожка освобождаются при выходе или отмене future.
    pub(super) async fn start(
        track: MediaStreamTrack,
        video: MediaStreamTrack,
        callbacks: ScreenShareCallbacks,
        video_timestamp_us: u64,
        anchor: Instant,
    ) -> Result<Rc<Self>, ScreenShareError> {
        let options = AudioContextOptions::new();
        options.set_sample_rate(48_000.0);
        options.set_latency_hint(&JsValue::from_str("interactive"));
        let context = AudioContext::new_with_context_options(&options).map_err(audio_error)?;
        let mut pending = PendingAudio {
            context: context.clone(),
            track: track.clone(),
            encoder: None,
            transferred: false,
        };
        if context.sample_rate() != 48_000.0 {
            return Err(ScreenShareError::new(
                "Браузер не поддерживает звук демонстрации с частотой 48 kHz.",
            ));
        }
        let worklet = context.audio_worklet().map_err(audio_error)?;
        JsFuture::from(
            worklet
                .unchecked_ref::<web_sys::Worklet>()
                .add_module("/audio/screen-audio-worklet.js?v=1")
                .map_err(audio_error)?,
        )
        .await
        .map_err(audio_error)?;

        let track_id = uuid::Uuid::new_v4();
        let closed = Rc::new(Cell::new(false));
        let weak = Rc::new(RefCell::new(Weak::<AudioGraph>::new()));
        let (mut sender, receiver) = mpsc::channel(1);
        let output_closed = closed.clone();
        let output_weak = weak.clone();
        let mut sequence = 0_u64;
        let output = Closure::wrap(Box::new(move |chunk: EncodedAudioChunk| {
            if output_closed.get() {
                return;
            }
            if let Some(graph) = output_weak.borrow().upgrade()
                && !whole_screen_selected(&graph.video)
            {
                graph.stop();
                return;
            }
            let destination = Uint8Array::new_with_length(chunk.byte_length());
            if let Err(error) = chunk.copy_to(&destination) {
                if let Some(graph) = output_weak.borrow().upgrade() {
                    graph.fail(error);
                }
                return;
            }
            let frame = EncodedScreenAudioFrame {
                track_id,
                sequence,
                timestamp_us: chunk.timestamp().max(0.0) as u64,
                duration_us: chunk.duration().unwrap_or(20_000.0).max(0.0) as u32,
                channels: 2,
                bytes: destination.to_vec(),
                closed: output_closed.clone(),
            };
            sequence = sequence.saturating_add(1);
            if sender.try_send(frame).is_err() && sequence.is_multiple_of(100) {
                debug!(%track_id, sequence, "dropped screen audio while sender was congested");
            }
        }) as Box<dyn FnMut(EncodedAudioChunk)>);
        let error_weak = weak.clone();
        let error = Closure::wrap(Box::new(move |error: JsValue| {
            if let Some(graph) = error_weak.borrow().upgrade() {
                graph.fail(error);
            }
        }) as Box<dyn FnMut(JsValue)>);
        let init = Object::new();
        set(&init, "output", output.as_ref())?;
        set(&init, "error", error.as_ref())?;
        let encoder = AudioEncoder::new(&init.into()).map_err(audio_error)?;
        pending.encoder = Some(encoder.clone());
        let config = Object::new();
        set(&config, "codec", &JsValue::from_str("opus"))?;
        set(&config, "sampleRate", &JsValue::from_f64(48_000.0))?;
        set(&config, "numberOfChannels", &JsValue::from_f64(2.0))?;
        set(&config, "bitrate", &JsValue::from_f64(128_000.0))?;
        encoder.configure(&config.into()).map_err(audio_error)?;

        let stream = MediaStream::new().map_err(audio_error)?;
        stream.add_track(&track);
        let source = context
            .create_media_stream_source(&stream)
            .map_err(audio_error)?;
        let options = AudioWorkletNodeOptions::new();
        options.set_number_of_inputs(1);
        options.set_number_of_outputs(1);
        options.set_output_channel_count(&Array::of1(&JsValue::from_f64(2.0)).into());
        let node = AudioWorkletNode::new_with_options(&context, "cheenhub-screen-audio", &options)
            .map_err(audio_error)?;
        let gain = context.create_gain().map_err(audio_error)?;
        gain.gain().set_value(0.0);
        source.connect_with_audio_node(&node).map_err(audio_error)?;
        node.connect_with_audio_node(&gain).map_err(audio_error)?;
        gain.connect_with_audio_node(&context.destination())
            .map_err(audio_error)?;
        let pcm_weak = weak.clone();
        let pcm = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Some(graph) = pcm_weak.borrow().upgrade() {
                graph.on_pcm(event);
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        let ended_weak = weak.clone();
        let ended = Closure::wrap(Box::new(move |_: web_sys::Event| {
            if let Some(graph) = ended_weak.borrow().upgrade() {
                graph.stop();
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        node.port()
            .map_err(audio_error)?
            .set_onmessage(Some(pcm.as_ref().unchecked_ref()));
        track.set_onended(Some(ended.as_ref().unchecked_ref()));
        let graph = Rc::new(AudioGraph {
            timestamp_base_us: video_timestamp_us as f64
                + anchor.elapsed().as_secs_f64() * 1_000_000.0
                - context.current_time() * 1_000_000.0,
            context,
            track,
            video,
            source,
            node,
            gain,
            encoder,
            closed,
            receiver: RefCell::new(Some(receiver)),
            callbacks,
            _pcm: pcm,
            _output: output,
            _error: error,
            _ended: ended,
        });
        *weak.borrow_mut() = Rc::downgrade(&graph);
        let capture = Rc::new(Self { graph });
        pending.transferred = true;
        JsFuture::from(capture.graph.context.resume().map_err(audio_error)?)
            .await
            .map_err(audio_error)?;
        let ended = Reflect::get(
            capture.graph.track.as_ref(),
            &JsValue::from_str("readyState"),
        )
        .ok()
        .and_then(|state| state.as_string())
        .is_some_and(|state| state == "ended");
        if capture.graph.closed.get() || ended {
            return Err(ScreenShareError::new(
                "Звук демонстрации завершился во время запуска.",
            ));
        }
        let own_audio_restricted = own_audio_restricted(&capture.graph.track);
        (capture.graph.callbacks.on_audio_state)(ScreenShareAudioState::Live {
            track_id,
            own_audio_restricted,
        });
        debug!(%track_id, own_audio_restricted, "started stereo screen audio capture");
        Ok(capture)
    }

    /// Передаёт единственному consumer ограниченную очередь закодированного аудио.
    pub(super) fn take_frames(&self) -> Option<mpsc::Receiver<EncodedScreenAudioFrame>> {
        self.graph.receiver.borrow_mut().take()
    }

    /// Немедленно отзывает кадры и освобождает track, encoder и аудиограф.
    pub(super) fn stop(&self) {
        self.graph.stop();
    }
}

impl Drop for BrowserScreenAudio {
    fn drop(&mut self) {
        self.graph.stop();
    }
}

impl AudioGraph {
    fn on_pcm(&self, event: MessageEvent) {
        if let Ok(port) = self.node.port() {
            let _ = port.post_message(&JsValue::from_str("consumed"));
        }
        if self.closed.get() {
            return;
        }
        if !whole_screen_selected(&self.video) {
            self.stop();
            return;
        }
        if self.encoder.queue_size() >= 4 {
            return;
        }
        if let Err(error) = self.encode_pcm(event.data()) {
            self.fail(error);
        }
    }

    fn encode_pcm(&self, message: JsValue) -> Result<(), JsValue> {
        let samples =
            Reflect::get(&message, &JsValue::from_str("samples"))?.dyn_into::<Float32Array>()?;
        if samples.length() != 1_920 {
            return Err(JsValue::from_str("invalid screen audio PCM size"));
        }
        let timestamp = Reflect::get(&message, &JsValue::from_str("timestampUs"))?
            .as_f64()
            .unwrap_or(0.0);
        let init = Object::new();
        Reflect::set(
            &init,
            &JsValue::from_str("format"),
            &JsValue::from_str("f32-planar"),
        )?;
        Reflect::set(
            &init,
            &JsValue::from_str("sampleRate"),
            &JsValue::from_f64(48_000.0),
        )?;
        Reflect::set(
            &init,
            &JsValue::from_str("numberOfFrames"),
            &JsValue::from_f64(960.0),
        )?;
        Reflect::set(
            &init,
            &JsValue::from_str("numberOfChannels"),
            &JsValue::from_f64(2.0),
        )?;
        Reflect::set(
            &init,
            &JsValue::from_str("timestamp"),
            &JsValue::from_f64((timestamp + self.timestamp_base_us).max(0.0)),
        )?;
        Reflect::set(&init, &JsValue::from_str("data"), samples.as_ref())?;
        let data = AudioData::new(&init.into())?;
        let result = self.encoder.encode(&data);
        let _ = data.close();
        result
    }

    fn fail(&self, error: JsValue) {
        if self.closed.get() {
            return;
        }
        warn!(error = %js_error_message(error), "screen audio capture failed");
        self.stop();
        (self.callbacks.on_audio_state)(ScreenShareAudioState::Unavailable);
    }

    fn stop(&self) {
        if self.closed.replace(true) {
            return;
        }
        self.track.set_onended(None);
        self.track.stop();
        if let Ok(port) = self.node.port() {
            port.set_onmessage(None);
        }
        let _ = self.source.disconnect();
        let _ = self.node.disconnect();
        let _ = self.gain.disconnect();
        let _ = self.encoder.close();
        if let Ok(promise) = self.context.close() {
            spawn_local(async move {
                if let Err(error) = JsFuture::from(promise).await {
                    debug!(error = %js_error_message(error), "screen audio context close failed");
                }
            });
        }
        (self.callbacks.on_audio_state)(ScreenShareAudioState::Off);
        debug!("stopped screen audio capture and released graph");
    }
}

fn set(object: &Object, name: &str, value: &JsValue) -> Result<(), ScreenShareError> {
    Reflect::set(object, &JsValue::from_str(name), value)
        .map(|_| ())
        .map_err(audio_error)
}

fn audio_error(error: JsValue) -> ScreenShareError {
    ScreenShareError::new(js_error_message(error))
}

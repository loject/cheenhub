//! Источник браузерных видео-кадров для WebCodecs.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::prelude::{debug, warn};
use futures_channel::oneshot;
use js_sys::{Array, Function, Object, Promise, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{MediaStream, MediaStreamTrack};

use super::backend::VideoEncodingError;
use super::web::{BrowserVideoFrame, global_constructor_available};

/// Reader кадров, который выбирает нативный track processor или совместимый video-element fallback.
pub(crate) struct BrowserVideoFrameReader {
    source: BrowserVideoFrameSource,
}

enum BrowserVideoFrameSource {
    TrackProcessor { reader: JsValue },
    VideoElement { state: Rc<VideoElementSourceState> },
}

/// Cloneable handle для остановки ожидающего browser frame source.
#[derive(Clone, Default)]
pub(crate) struct BrowserVideoFrameReaderHandle {
    video_element: Option<Rc<VideoElementSourceState>>,
}

impl BrowserVideoFrameReaderHandle {
    /// Останавливает fallback и освобождает привязанный `HTMLVideoElement`.
    pub(crate) fn stop(&self) {
        if let Some(state) = &self.video_element {
            state.stop();
        }
    }
}

impl BrowserVideoFrameReader {
    /// Возвращает доступность хотя бы одного browser frame source.
    pub(crate) fn capture_available() -> bool {
        global_constructor_available("MediaStreamTrackProcessor")
            || video_element_frame_callback_available()
    }

    /// Создает reader для браузерного `MediaStream`.
    pub(crate) async fn from_stream(
        track: &MediaStreamTrack,
        stream: &MediaStream,
    ) -> Result<Self, VideoEncodingError> {
        if global_constructor_available("MediaStreamTrackProcessor") {
            match track_processor_reader(track) {
                Ok(reader) => {
                    debug!(
                        frame_source = "media-stream-track-processor",
                        "browser video frame source selected"
                    );
                    return Ok(Self {
                        source: BrowserVideoFrameSource::TrackProcessor { reader },
                    });
                }
                Err(error) if video_element_frame_callback_available() => {
                    warn!(
                        %error,
                        fallback = "html-video-element",
                        "browser track processor failed; using compatible video frame source"
                    );
                }
                Err(error) => return Err(error),
            }
        }

        let state = VideoElementSourceState::start(track, stream).await?;
        debug!(
            frame_source = "html-video-element",
            "browser video frame source selected"
        );
        Ok(Self {
            source: BrowserVideoFrameSource::VideoElement { state },
        })
    }

    /// Возвращает handle для явной остановки ожидающего fallback callback.
    pub(crate) fn handle(&self) -> BrowserVideoFrameReaderHandle {
        BrowserVideoFrameReaderHandle {
            video_element: match &self.source {
                BrowserVideoFrameSource::TrackProcessor { .. } => None,
                BrowserVideoFrameSource::VideoElement { state } => Some(state.clone()),
            },
        }
    }

    /// Читает следующий кадр или `None`, если поток завершен.
    pub(crate) async fn read(&self) -> Result<Option<BrowserVideoFrame>, VideoEncodingError> {
        match &self.source {
            BrowserVideoFrameSource::TrackProcessor { reader } => {
                let read = read_stream_chunk(reader).await?;
                if read.done {
                    Ok(None)
                } else {
                    Ok(Some(BrowserVideoFrame::from_js_value(&read.value)))
                }
            }
            BrowserVideoFrameSource::VideoElement { state } => state.read().await,
        }
    }
}

impl Drop for BrowserVideoFrameReader {
    fn drop(&mut self) {
        if let BrowserVideoFrameSource::VideoElement { state } = &self.source {
            state.stop();
        }
    }
}

type PendingFrame = Result<Option<BrowserVideoFrame>, VideoEncodingError>;
type VideoFrameCallbackClosure = Closure<dyn FnMut(f64, JsValue)>;

struct VideoElementSourceState {
    video: JsValue,
    track: MediaStreamTrack,
    stopped: Cell<bool>,
    callback_id: Cell<Option<i32>>,
    pending: RefCell<Option<oneshot::Sender<PendingFrame>>>,
    pending_frame_closure: RefCell<Option<VideoFrameCallbackClosure>>,
    track_ended_closure: RefCell<Option<Closure<dyn FnMut()>>>,
}

impl VideoElementSourceState {
    async fn start(
        track: &MediaStreamTrack,
        stream: &MediaStream,
    ) -> Result<Rc<Self>, VideoEncodingError> {
        if !video_element_frame_callback_available() {
            return Err(VideoEncodingError::unsupported(
                "HTMLVideoElement.requestVideoFrameCallback недоступен.",
            ));
        }

        let document = web_sys::window()
            .and_then(|window| window.document())
            .ok_or_else(|| VideoEncodingError::unavailable("Документ браузера недоступен."))?;
        let video: JsValue = document
            .create_element("video")
            .map_err(video_error)?
            .into();
        set_property(&video, "srcObject", stream.as_ref())?;
        set_property(&video, "muted", &JsValue::TRUE)?;
        set_property(&video, "autoplay", &JsValue::TRUE)?;
        set_property(&video, "playsInline", &JsValue::TRUE)?;
        play_video(&video).await?;

        let state = Rc::new(Self {
            video,
            track: track.clone(),
            stopped: Cell::new(false),
            callback_id: Cell::new(None),
            pending: RefCell::new(None),
            pending_frame_closure: RefCell::new(None),
            track_ended_closure: RefCell::new(None),
        });
        state.install_track_ended_handler()?;
        Ok(state)
    }

    fn install_track_ended_handler(self: &Rc<Self>) -> Result<(), VideoEncodingError> {
        let weak_state = Rc::downgrade(self);
        let closure = Closure::wrap(Box::new(move || {
            if let Some(state) = weak_state.upgrade() {
                state.stop();
            }
        }) as Box<dyn FnMut()>);
        set_property(
            self.track.as_ref(),
            "onended",
            closure.as_ref().unchecked_ref(),
        )?;
        self.track_ended_closure.replace(Some(closure));
        Ok(())
    }

    async fn read(self: &Rc<Self>) -> Result<Option<BrowserVideoFrame>, VideoEncodingError> {
        if self.stopped.get() {
            return Ok(None);
        }
        if self.pending.borrow().is_some() {
            return Err(VideoEncodingError::unavailable(
                "Параллельное чтение HTMLVideoElement не поддерживается.",
            ));
        }

        let (sender, receiver) = oneshot::channel();
        self.pending.replace(Some(sender));
        let weak_state = Rc::downgrade(self);
        let closure = Closure::wrap(Box::new(move |now_ms: f64, metadata: JsValue| {
            let Some(state) = weak_state.upgrade() else {
                return;
            };
            state.callback_id.set(None);
            let _registered_closure = state.pending_frame_closure.borrow_mut().take();
            let Some(sender) = state.pending.borrow_mut().take() else {
                return;
            };
            if state.stopped.get() {
                let _ = sender.send(Ok(None));
                return;
            }
            let timestamp_us = frame_timestamp_us(now_ms, &metadata);
            let _ = sender.send(video_frame_from_element(&state.video, timestamp_us).map(Some));
        }) as Box<dyn FnMut(f64, JsValue)>);
        self.pending_frame_closure.replace(Some(closure));
        let callback = self
            .pending_frame_closure
            .borrow()
            .as_ref()
            .map(|closure| closure.as_ref().unchecked_ref::<Function>().clone())
            .ok_or_else(|| {
                VideoEncodingError::unavailable("Callback браузерного видео-кадра не установлен.")
            })?;

        let callback_id =
            request_video_frame_callback(&self.video, &callback).inspect_err(|_| {
                self.pending_frame_closure.borrow_mut().take();
                self.pending.borrow_mut().take();
            })?;
        self.callback_id.set(Some(callback_id));

        receiver.await.map_err(|_| {
            VideoEncodingError::unavailable("Ожидание браузерного видео-кадра было прервано.")
        })?
    }

    fn stop(&self) {
        if self.stopped.replace(true) {
            return;
        }
        if let Some(callback_id) = self.callback_id.take() {
            let _ = cancel_video_frame_callback(&self.video, callback_id);
        }
        if let Some(sender) = self.pending.borrow_mut().take() {
            let _ = sender.send(Ok(None));
        }
        self.pending_frame_closure.borrow_mut().take();
        let _ = call_method0(&self.video, "pause");
        let _ = set_property(&self.video, "srcObject", &JsValue::NULL);
        let _ = set_property(self.track.as_ref(), "onended", &JsValue::NULL);
        self.track_ended_closure.borrow_mut().take();
    }
}

fn video_element_frame_callback_available() -> bool {
    let Ok(constructor) = Reflect::get(&js_sys::global(), &JsValue::from_str("HTMLVideoElement"))
    else {
        return false;
    };
    let Ok(prototype) = Reflect::get(&constructor, &JsValue::from_str("prototype")) else {
        return false;
    };
    Reflect::get(&prototype, &JsValue::from_str("requestVideoFrameCallback"))
        .ok()
        .and_then(|value| value.dyn_into::<Function>().ok())
        .is_some()
}

fn track_processor_reader(track: &MediaStreamTrack) -> Result<JsValue, VideoEncodingError> {
    let constructor = global_function("MediaStreamTrackProcessor")?;
    let init = Object::new();
    set_property(init.as_ref(), "track", track.as_ref())?;
    let arguments = Array::of1(&init);
    let processor = Reflect::construct(&constructor, &arguments).map_err(video_error)?;
    let readable = Reflect::get(&processor, &JsValue::from_str("readable")).map_err(video_error)?;
    call_method0(&readable, "getReader")
}

struct StreamRead {
    done: bool,
    value: JsValue,
}

async fn read_stream_chunk(reader: &JsValue) -> Result<StreamRead, VideoEncodingError> {
    let result = JsFuture::from(
        call_method0(reader, "read")?
            .dyn_into::<Promise>()
            .map_err(|_| {
                VideoEncodingError::unavailable("ReadableStream reader.read не вернул Promise.")
            })?,
    )
    .await
    .map_err(video_error)?;
    let done = Reflect::get(&result, &JsValue::from_str("done"))
        .map_err(video_error)?
        .as_bool()
        .unwrap_or(false);
    let value = Reflect::get(&result, &JsValue::from_str("value")).map_err(video_error)?;
    Ok(StreamRead { done, value })
}

async fn play_video(video: &JsValue) -> Result<(), VideoEncodingError> {
    let promise = call_method0(video, "play")?
        .dyn_into::<Promise>()
        .map_err(|_| VideoEncodingError::unavailable("HTMLVideoElement.play не вернул Promise."))?;
    JsFuture::from(promise).await.map_err(video_error)?;
    Ok(())
}

fn request_video_frame_callback(
    video: &JsValue,
    callback: &Function,
) -> Result<i32, VideoEncodingError> {
    call_method1(video, "requestVideoFrameCallback", callback.as_ref())?
        .as_f64()
        .map(|id| id as i32)
        .ok_or_else(|| VideoEncodingError::unavailable("requestVideoFrameCallback не вернул id."))
}

fn cancel_video_frame_callback(
    video: &JsValue,
    callback_id: i32,
) -> Result<(), VideoEncodingError> {
    call_method1(
        video,
        "cancelVideoFrameCallback",
        &JsValue::from_f64(f64::from(callback_id)),
    )?;
    Ok(())
}

fn video_frame_from_element(
    video: &JsValue,
    timestamp_us: f64,
) -> Result<BrowserVideoFrame, VideoEncodingError> {
    let constructor = global_function("VideoFrame")?;
    let init = Object::new();
    set_property(init.as_ref(), "timestamp", &JsValue::from_f64(timestamp_us))?;
    let arguments = Array::new();
    arguments.push(video);
    arguments.push(&init);
    Reflect::construct(&constructor, &arguments)
        .map(|frame| BrowserVideoFrame::from_js_value(&frame))
        .map_err(video_error)
}

fn frame_timestamp_us(now_ms: f64, metadata: &JsValue) -> f64 {
    Reflect::get(metadata, &JsValue::from_str("mediaTime"))
        .ok()
        .and_then(|value| value.as_f64())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|seconds| seconds * 1_000_000.0)
        .unwrap_or_else(|| now_ms.max(0.0) * 1_000.0)
}

fn global_function(name: &str) -> Result<Function, VideoEncodingError> {
    Reflect::get(&js_sys::global(), &JsValue::from_str(name))
        .map_err(video_error)?
        .dyn_into::<Function>()
        .map_err(|_| VideoEncodingError::unavailable(format!("{name} недоступен.")))
}

fn call_method0(target: &JsValue, name: &str) -> Result<JsValue, VideoEncodingError> {
    method(target, name)?.call0(target).map_err(video_error)
}

fn call_method1(
    target: &JsValue,
    name: &str,
    argument: &JsValue,
) -> Result<JsValue, VideoEncodingError> {
    method(target, name)?
        .call1(target, argument)
        .map_err(video_error)
}

fn method(target: &JsValue, name: &str) -> Result<Function, VideoEncodingError> {
    Reflect::get(target, &JsValue::from_str(name))
        .map_err(video_error)?
        .dyn_into::<Function>()
        .map_err(|_| VideoEncodingError::unavailable(format!("{name} недоступен.")))
}

fn set_property(target: &JsValue, name: &str, value: &JsValue) -> Result<(), VideoEncodingError> {
    Reflect::set(target, &JsValue::from_str(name), value)
        .map(|_| ())
        .map_err(video_error)
}

fn video_error(error: JsValue) -> VideoEncodingError {
    VideoEncodingError::unavailable(
        error
            .dyn_ref::<js_sys::Error>()
            .map(js_sys::Error::message)
            .and_then(|message| message.as_string())
            .filter(|message| !message.is_empty())
            .or_else(|| error.as_string())
            .unwrap_or_else(|| "Неизвестная ошибка браузерного источника видео.".to_owned()),
    )
}

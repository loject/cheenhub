//! Browser-backend renderer'а видео участника.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::prelude::{debug, warn};
use gloo_timers::future::TimeoutFuture;
use js_sys::{Array, Function, Object, Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use super::ParticipantVideoFrame;
use super::backend::{
    ParticipantVideoBackend, ParticipantVideoRenderError, ParticipantVideoRenderer,
};

const VIDEO_CALLBACK_RELEASE_DELAY_MS: u32 = 250;

/// Browser-реализация на основе WebCodecs и canvas.
pub(crate) struct WebParticipantVideoBackend;

impl ParticipantVideoBackend for WebParticipantVideoBackend {
    fn create_renderer(
        &self,
        target_id: String,
        user_id: String,
        source_label: &'static str,
    ) -> Result<Rc<dyn ParticipantVideoRenderer>, ParticipantVideoRenderError> {
        BrowserParticipantVideoRenderer::new(target_id, user_id, source_label)
            .map(|renderer| Rc::new(renderer) as Rc<dyn ParticipantVideoRenderer>)
    }
}

struct BrowserParticipantVideoRenderer {
    decoder: VideoDecoder,
    canvas: Rc<RefCell<Option<JsValue>>>,
    source_label: &'static str,
    closed: Rc<Cell<bool>>,
    received_key_frame: Rc<Cell<bool>>,
    waiting_key_frame_logged: Rc<Cell<bool>>,
    output_closure: RefCell<Option<Closure<dyn FnMut(VideoFrame)>>>,
    error_closure: RefCell<Option<Closure<dyn FnMut(JsValue)>>>,
}

impl BrowserParticipantVideoRenderer {
    fn new(
        target_id: String,
        user_id: String,
        source_label: &'static str,
    ) -> Result<Self, ParticipantVideoRenderError> {
        let canvas = Rc::new(RefCell::new(None));
        let closed = Rc::new(Cell::new(false));
        let output_canvas = canvas.clone();
        let output_closed = closed.clone();
        let output_target_id = target_id.clone();
        let output_user_id = user_id.clone();
        let output_source_label = source_label;
        let output_closure = Closure::wrap(Box::new(move |frame: VideoFrame| {
            if output_closed.get() {
                let _ = frame.close();
                return;
            }
            let canvas = match get_or_create_canvas(&output_canvas, &output_target_id) {
                Ok(canvas) => canvas,
                Err(error) => {
                    warn!(
                        %error,
                        sender_user_id = %output_user_id,
                        source = output_source_label,
                        "failed to prepare participant video canvas"
                    );
                    let _ = frame.close();
                    return;
                }
            };

            if let Err(error) = draw_video_frame(&canvas, &frame) {
                warn!(
                    error = %js_error_message(error),
                    sender_user_id = %output_user_id,
                    source = output_source_label,
                    "failed to draw participant video frame"
                );
            }
            let _ = frame.close();
        }) as Box<dyn FnMut(VideoFrame)>);
        let error_closed = closed.clone();
        let error_closure = Closure::wrap(Box::new(move |error: JsValue| {
            if error_closed.get() {
                return;
            }
            warn!(
                error = %js_error_message(error),
                sender_user_id = %user_id,
                source = source_label,
                "participant video decoder failed"
            );
        }) as Box<dyn FnMut(JsValue)>);

        let init = Object::new();
        Reflect::set(&init, &JsValue::from_str("output"), output_closure.as_ref())
            .map_err(render_error)?;
        Reflect::set(&init, &JsValue::from_str("error"), error_closure.as_ref())
            .map_err(render_error)?;
        let decoder = VideoDecoder::new(&init.into()).map_err(render_error)?;
        decoder.configure(&decoder_config()).map_err(render_error)?;

        Ok(Self {
            decoder,
            canvas,
            source_label,
            closed,
            received_key_frame: Rc::new(Cell::new(false)),
            waiting_key_frame_logged: Rc::new(Cell::new(false)),
            output_closure: RefCell::new(Some(output_closure)),
            error_closure: RefCell::new(Some(error_closure)),
        })
    }
}

impl ParticipantVideoRenderer for BrowserParticipantVideoRenderer {
    fn decode(&self, frame: &ParticipantVideoFrame) -> Result<(), ParticipantVideoRenderError> {
        if self.closed.get() {
            return Ok(());
        }
        if frame.bytes.is_empty() {
            return Ok(());
        }
        if !self.received_key_frame.get() && !frame.key_frame {
            if !self.waiting_key_frame_logged.replace(true) {
                debug!(
                    sender_user_id = %frame.sender_user_id,
                    sequence = frame.sequence,
                    source = self.source_label,
                    "waiting for participant video key frame before decoding"
                );
            }
            return Ok(());
        }
        if frame.key_frame && !self.received_key_frame.replace(true) {
            debug!(
                sender_user_id = %frame.sender_user_id,
                sequence = frame.sequence,
                payload_bytes = frame.bytes.len(),
                source = self.source_label,
                "received first participant video key frame for decoder"
            );
        }

        let chunk = encoded_video_chunk(frame).map_err(render_error)?;
        self.decoder.decode(&chunk).map_err(render_error)
    }

    fn close(&self) {
        if self.closed.replace(true) {
            return;
        }
        if let Err(error) = self.decoder.close() {
            warn!(
                error = %js_error_message(error),
                source = self.source_label,
                "failed to close participant video decoder"
            );
        }
        if let Some(canvas) = self.canvas.borrow_mut().take() {
            let _ = remove_canvas(&canvas);
        }
        let output_closure = self.output_closure.borrow_mut().take();
        let error_closure = self.error_closure.borrow_mut().take();
        defer_video_callbacks_drop(output_closure, error_closure);
        debug!(
            source = self.source_label,
            "closed participant video renderer"
        );
    }
}

impl Drop for BrowserParticipantVideoRenderer {
    fn drop(&mut self) {
        self.close();
    }
}

fn defer_video_callbacks_drop(
    output_closure: Option<Closure<dyn FnMut(VideoFrame)>>,
    error_closure: Option<Closure<dyn FnMut(JsValue)>>,
) {
    let Some(output_closure) = output_closure else {
        return;
    };
    let Some(error_closure) = error_closure else {
        return;
    };

    spawn_local(async move {
        let callbacks = (output_closure, error_closure);
        TimeoutFuture::new(VIDEO_CALLBACK_RELEASE_DELAY_MS).await;
        drop(callbacks);
    });
}

fn decoder_config() -> JsValue {
    let object = Object::new();
    set_property(&object, "codec", &JsValue::from_str("vp09.00.10.08"));
    object.into()
}

fn encoded_video_chunk(frame: &ParticipantVideoFrame) -> Result<EncodedVideoChunk, JsValue> {
    let data = Uint8Array::from(frame.bytes.as_slice());
    let init = Object::new();
    Reflect::set(
        &init,
        &JsValue::from_str("type"),
        &JsValue::from_str(if frame.key_frame { "key" } else { "delta" }),
    )?;
    Reflect::set(
        &init,
        &JsValue::from_str("timestamp"),
        &JsValue::from_f64(frame.timestamp_us as f64),
    )?;
    Reflect::set(
        &init,
        &JsValue::from_str("duration"),
        &JsValue::from_f64(f64::from(frame.duration_us)),
    )?;
    Reflect::set(&init, &JsValue::from_str("data"), data.as_ref())?;
    EncodedVideoChunk::new(&init.into())
}

fn create_canvas_in_target(target_id: &str) -> Result<JsValue, ParticipantVideoRenderError> {
    let window = web_sys::window().ok_or_else(|| {
        ParticipantVideoRenderError::new("Окно браузера недоступно для отображения видео.")
    })?;
    let document = window.document().ok_or_else(|| {
        ParticipantVideoRenderError::new("Документ браузера недоступен для отображения видео.")
    })?;
    let target = document
        .get_element_by_id(target_id)
        .ok_or_else(|| ParticipantVideoRenderError::new("Контейнер видео недоступен."))?;
    let canvas = document.create_element("canvas").map_err(render_error)?;
    set_property(canvas.as_ref(), "ariaHidden", &JsValue::from_str("true"));
    set_property(
        canvas.as_ref(),
        "className",
        &JsValue::from_str("absolute inset-0 h-full w-full bg-zinc-950 object-cover"),
    );
    target.append_child(&canvas).map_err(render_error)?;

    Ok(canvas.into())
}

fn get_or_create_canvas(
    canvas: &Rc<RefCell<Option<JsValue>>>,
    target_id: &str,
) -> Result<JsValue, ParticipantVideoRenderError> {
    if let Some(canvas) = canvas.borrow().as_ref() {
        return Ok(canvas.clone());
    }

    let next_canvas = create_canvas_in_target(target_id)?;
    canvas.borrow_mut().replace(next_canvas.clone());
    Ok(next_canvas)
}

fn draw_video_frame(canvas: &JsValue, frame: &VideoFrame) -> Result<(), JsValue> {
    let width = frame.display_width().max(1);
    let height = frame.display_height().max(1);
    set_property(canvas, "width", &JsValue::from_f64(f64::from(width)));
    set_property(canvas, "height", &JsValue::from_f64(f64::from(height)));

    let get_context =
        Reflect::get(canvas, &JsValue::from_str("getContext"))?.dyn_into::<Function>()?;
    let context = get_context.call1(canvas, &JsValue::from_str("2d"))?;
    let draw_image =
        Reflect::get(&context, &JsValue::from_str("drawImage"))?.dyn_into::<Function>()?;
    let args = Array::new();
    args.push(frame.as_ref());
    args.push(&JsValue::from_f64(0.0));
    args.push(&JsValue::from_f64(0.0));
    args.push(&JsValue::from_f64(f64::from(width)));
    args.push(&JsValue::from_f64(f64::from(height)));
    Reflect::apply(&draw_image, &context, &args).map(|_| ())
}

fn remove_canvas(canvas: &JsValue) -> Result<(), JsValue> {
    let remove = Reflect::get(canvas, &JsValue::from_str("remove"))?.dyn_into::<Function>()?;
    remove.call0(canvas).map(|_| ())
}

fn render_error(error: JsValue) -> ParticipantVideoRenderError {
    ParticipantVideoRenderError::new(js_error_message(error))
}

fn js_error_message(error: JsValue) -> String {
    if let Some(message) = Reflect::get(&error, &JsValue::from_str("message"))
        .ok()
        .and_then(|value| value.as_string())
        .filter(|value| !value.is_empty())
    {
        return message;
    }

    if let Some(value) = error.as_string().filter(|value| !value.is_empty()) {
        return value;
    }

    "Неизвестная ошибка браузера.".to_string()
}

fn set_property(object: &JsValue, name: &str, value: &JsValue) {
    let _ = Reflect::set(object, &JsValue::from_str(name), value);
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = VideoDecoder)]
    #[derive(Clone)]
    type VideoDecoder;

    #[wasm_bindgen(constructor, catch, js_class = VideoDecoder)]
    fn new(init: &JsValue) -> Result<VideoDecoder, JsValue>;

    #[wasm_bindgen(method, catch, js_name = configure)]
    fn configure(this: &VideoDecoder, config: &JsValue) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = decode)]
    fn decode(this: &VideoDecoder, chunk: &EncodedVideoChunk) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = close)]
    fn close(this: &VideoDecoder) -> Result<(), JsValue>;

    #[wasm_bindgen(js_name = EncodedVideoChunk)]
    type EncodedVideoChunk;

    #[wasm_bindgen(constructor, catch, js_class = EncodedVideoChunk)]
    fn new(init: &JsValue) -> Result<EncodedVideoChunk, JsValue>;

    #[wasm_bindgen(js_name = VideoFrame)]
    type VideoFrame;

    #[wasm_bindgen(method, getter, js_name = displayWidth)]
    fn display_width(this: &VideoFrame) -> u32;

    #[wasm_bindgen(method, getter, js_name = displayHeight)]
    fn display_height(this: &VideoFrame) -> u32;

    #[wasm_bindgen(method, catch, js_name = close)]
    fn close(this: &VideoFrame) -> Result<(), JsValue>;
}

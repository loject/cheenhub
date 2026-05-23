//! Browser screen sharing video renderer backend.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::prelude::{debug, warn};
use js_sys::{Array, Function, Object, Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use super::InboundScreenFrame;
use super::backend::{ScreenVideoBackend, ScreenVideoRenderError, ScreenVideoRenderer};

/// Browser implementation backed by WebCodecs and a canvas.
pub(crate) struct WebScreenVideoBackend;

impl ScreenVideoBackend for WebScreenVideoBackend {
    fn create_renderer(
        &self,
        target_id: String,
        user_id: String,
    ) -> Result<Rc<dyn ScreenVideoRenderer>, ScreenVideoRenderError> {
        BrowserScreenVideoRenderer::new(target_id, user_id)
            .map(|renderer| Rc::new(renderer) as Rc<dyn ScreenVideoRenderer>)
    }
}

struct BrowserScreenVideoRenderer {
    decoder: VideoDecoder,
    canvas: Rc<RefCell<Option<JsValue>>>,
    received_key_frame: Rc<Cell<bool>>,
    waiting_key_frame_logged: Rc<Cell<bool>>,
    _output_closure: Closure<dyn FnMut(VideoFrame)>,
    _error_closure: Closure<dyn FnMut(JsValue)>,
}

impl BrowserScreenVideoRenderer {
    fn new(target_id: String, user_id: String) -> Result<Self, ScreenVideoRenderError> {
        let canvas = Rc::new(RefCell::new(None));
        let output_canvas = canvas.clone();
        let output_target_id = target_id.clone();
        let output_user_id = user_id.clone();
        let output_closure = Closure::wrap(Box::new(move |frame: VideoFrame| {
            let canvas = match get_or_create_canvas(&output_canvas, &output_target_id) {
                Ok(canvas) => canvas,
                Err(error) => {
                    warn!(
                        %error,
                        sender_user_id = %output_user_id,
                        "failed to prepare screen sharing canvas"
                    );
                    let _ = frame.close();
                    return;
                }
            };

            if let Err(error) = draw_video_frame(&canvas, &frame) {
                warn!(
                    error = %js_error_message(error),
                    sender_user_id = %output_user_id,
                    "failed to draw screen sharing frame"
                );
            }
            let _ = frame.close();
        }) as Box<dyn FnMut(VideoFrame)>);
        let error_closure = Closure::wrap(Box::new(move |error: JsValue| {
            warn!(
                error = %js_error_message(error),
                sender_user_id = %user_id,
                "screen sharing video decoder failed"
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
            received_key_frame: Rc::new(Cell::new(false)),
            waiting_key_frame_logged: Rc::new(Cell::new(false)),
            _output_closure: output_closure,
            _error_closure: error_closure,
        })
    }
}

impl ScreenVideoRenderer for BrowserScreenVideoRenderer {
    fn decode(&self, frame: &InboundScreenFrame) -> Result<(), ScreenVideoRenderError> {
        if frame.bytes.is_empty() {
            return Ok(());
        }
        if !self.received_key_frame.get() && !frame.key_frame {
            if !self.waiting_key_frame_logged.replace(true) {
                debug!(
                    sender_user_id = %frame.sender_user_id,
                    sequence = frame.sequence,
                    "waiting for screen sharing key frame before decoding"
                );
            }
            return Ok(());
        }
        if frame.key_frame && !self.received_key_frame.replace(true) {
            debug!(
                sender_user_id = %frame.sender_user_id,
                sequence = frame.sequence,
                payload_bytes = frame.bytes.len(),
                "received first screen sharing key frame for decoder"
            );
        }

        let chunk = encoded_video_chunk(frame).map_err(render_error)?;
        self.decoder.decode(&chunk).map_err(render_error)
    }

    fn close(&self) {
        if let Err(error) = self.decoder.close() {
            warn!(
                error = %js_error_message(error),
                "failed to close screen sharing video decoder"
            );
        }
        if let Some(canvas) = self.canvas.borrow_mut().take() {
            let _ = remove_canvas(&canvas);
        }
    }
}

fn decoder_config() -> JsValue {
    let object = Object::new();
    set_property(&object, "codec", &JsValue::from_str("vp09.00.10.08"));
    object.into()
}

fn encoded_video_chunk(frame: &InboundScreenFrame) -> Result<EncodedVideoChunk, JsValue> {
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

fn create_canvas_in_target(target_id: &str) -> Result<JsValue, ScreenVideoRenderError> {
    let window = web_sys::window().ok_or_else(|| {
        ScreenVideoRenderError::new("Окно браузера недоступно для отображения демонстрации экрана.")
    })?;
    let document = window.document().ok_or_else(|| {
        ScreenVideoRenderError::new(
            "Документ браузера недоступен для отображения демонстрации экрана.",
        )
    })?;
    let target = document
        .get_element_by_id(target_id)
        .ok_or_else(|| ScreenVideoRenderError::new("Контейнер демонстрации экрана недоступен."))?;
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
) -> Result<JsValue, ScreenVideoRenderError> {
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

fn render_error(error: JsValue) -> ScreenVideoRenderError {
    ScreenVideoRenderError::new(js_error_message(error))
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

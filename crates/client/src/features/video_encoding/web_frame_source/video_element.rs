//! Источник кадров через скрытый `HTMLVideoElement` для браузерного fallback.

use super::*;

type PendingFrame = Result<Option<BrowserVideoFrame>, VideoEncodingError>;
type VideoFrameCallbackClosure = Closure<dyn FnMut(f64, JsValue)>;

pub(super) struct VideoElementSourceState {
    video: JsValue,
    track: MediaStreamTrack,
    stopped: Cell<bool>,
    callback_id: Cell<Option<i32>>,
    pending: RefCell<Option<oneshot::Sender<PendingFrame>>>,
    pending_frame_closure: RefCell<Option<VideoFrameCallbackClosure>>,
    track_ended_closure: RefCell<Option<Closure<dyn FnMut()>>>,
    trace_frame_count: Cell<u64>,
}

impl VideoElementSourceState {
    pub(super) async fn start(
        track: &MediaStreamTrack,
        stream: &MediaStream,
        cancellation: Option<oneshot::Receiver<()>>,
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
        let mut pending_video = PendingVideoElement {
            video,
            transferred: false,
        };
        set_property(&pending_video.video, "srcObject", stream.as_ref())?;
        set_property(&pending_video.video, "muted", &JsValue::TRUE)?;
        set_property(&pending_video.video, "autoplay", &JsValue::TRUE)?;
        set_property(&pending_video.video, "playsInline", &JsValue::TRUE)?;
        call_method2(
            &pending_video.video,
            "setAttribute",
            &JsValue::from_str("style"),
            &JsValue::from_str(
                "position:fixed;top:12px;right:12px;width:320px;max-width:35vw;opacity:0;pointer-events:none;",
            ),
        )?;
        call_method2(
            &pending_video.video,
            "setAttribute",
            &JsValue::from_str("aria-hidden"),
            &JsValue::from_str("true"),
        )?;
        if let Ok(body) = Reflect::get(document.as_ref(), &JsValue::from_str("body"))
            && !body.is_null()
            && !body.is_undefined()
        {
            call_method1(&body, "appendChild", &pending_video.video)?;
        }
        if let Some(cancellation) = cancellation {
            use futures_util::future::{Either, select};
            let play = play_video(&pending_video.video).boxed_local();
            let cancelled = async move {
                let _ = cancellation.await;
                Err(VideoEncodingError::unavailable(
                    "Запуск HTMLVideoElement был отменён.",
                ))
            }
            .boxed_local();
            match select(play, cancelled).await {
                Either::Left((result, _)) => result?,
                Either::Right((result, _)) => result?,
            }
        } else {
            play_video(&pending_video.video).await?;
        }

        let state = Rc::new(Self {
            video: pending_video.video.clone(),
            track: track.clone(),
            stopped: Cell::new(false),
            callback_id: Cell::new(None),
            pending: RefCell::new(None),
            pending_frame_closure: RefCell::new(None),
            track_ended_closure: RefCell::new(None),
            trace_frame_count: Cell::new(0),
        });
        state.install_track_ended_handler()?;
        pending_video.transferred = true;
        Ok(state)
    }

    fn install_track_ended_handler(self: &Rc<Self>) -> Result<(), VideoEncodingError> {
        let weak_state = Rc::downgrade(self);
        let closure = Closure::wrap(Box::new(move || {
            if let Some(state) = weak_state.upgrade() {
                state.stop();
            }
        }) as Box<dyn FnMut()>);
        self.track
            .add_event_listener_with_callback("ended", closure.as_ref().unchecked_ref())
            .map_err(video_error)?;
        self.track_ended_closure.replace(Some(closure));
        Ok(())
    }

    pub(super) async fn read(
        self: &Rc<Self>,
    ) -> Result<Option<BrowserVideoFrame>, VideoEncodingError> {
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
            // Сохраняем callback до выхода JavaScript из него; следующий read заменит его позже.
            let Some(sender) = state.pending.borrow_mut().take() else {
                return;
            };
            if state.stopped.get() {
                let _ = sender.send(Ok(None));
                return;
            }
            let timestamp_us = frame_timestamp_us(now_ms, &metadata);
            let trace_frame = state.trace_frame_count.get();
            state.trace_frame_count.set(trace_frame.saturating_add(1));
            if trace_frame.is_multiple_of(15) {
                let presented_frames =
                    Reflect::get(&metadata, &JsValue::from_str("presentedFrames"))
                        .ok()
                        .and_then(|value| value.as_f64())
                        .unwrap_or(-1.0);
                debug!(
                    now_ms,
                    timestamp_us, presented_frames, "screen share latency trace: raw video frame"
                );
            }
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

    pub(super) fn stop(&self) {
        if self.stopped.replace(true) {
            return;
        }
        debug!("stopping HTMLVideoElement frame source");
        if let Some(callback_id) = self.callback_id.take() {
            if let Err(error) = cancel_video_frame_callback(&self.video, callback_id) {
                warn!(%error, "failed to cancel browser video frame callback");
            } else {
                debug!(callback_id, "cancelled browser video frame callback");
            }
        }
        if let Some(sender) = self.pending.borrow_mut().take() {
            let _ = sender.send(Ok(None));
        }
        self.pending_frame_closure.borrow_mut().take();
        cleanup_video_element(&self.video);
        if let Some(closure) = self.track_ended_closure.borrow().as_ref() {
            let _ = self
                .track
                .remove_event_listener_with_callback("ended", closure.as_ref().unchecked_ref());
        }
        self.track_ended_closure.borrow_mut().take();
    }
}

struct PendingVideoElement {
    video: JsValue,
    transferred: bool,
}

impl Drop for PendingVideoElement {
    fn drop(&mut self) {
        if self.transferred {
            return;
        }
        cleanup_video_element(&self.video);
        debug!("removed temporary browser video element after failed startup");
    }
}

fn cleanup_video_element(video: &JsValue) {
    for (operation, result) in [
        ("pause", call_method0(video, "pause").map(|_| ())),
        ("remove", call_method0(video, "remove").map(|_| ())),
        (
            "clear srcObject",
            set_property(video, "srcObject", &JsValue::NULL),
        ),
    ] {
        if let Err(error) = result {
            warn!(%error, operation, "failed to clean up HTMLVideoElement resource");
        }
    }
    debug!("removed HTMLVideoElement and cleared its capture source");
}

pub(super) fn video_element_frame_callback_available() -> bool {
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

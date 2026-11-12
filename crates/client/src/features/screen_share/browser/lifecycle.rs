//! Единый владелец ресурсов одной browser screen capture.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use dioxus::prelude::{debug, warn};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use web_sys::{Event, MediaStream, MediaStreamTrack};

use crate::features::video_encoding::{
    BrowserVideoEncoder, BrowserVideoEncoderHandle, BrowserVideoFrameReaderHandle,
    VideoFrameEncoder,
};

use super::super::backend::{ScreenShareCallbacks, ScreenShareStartControl};
use super::super::browser_capture::stop_screen_stream;
use super::super::browser_errors::js_error_message;
use super::audio::BrowserScreenAudio;

type EndedHandler = Closure<dyn FnMut(Event)>;

#[cfg(test)]
mod tests;

/// Владеет исходным потоком и всеми ресурсами его видеосессии.
pub(super) struct BrowserCaptureLifecycle {
    stream: MediaStream,
    video_track: MediaStreamTrack,
    callbacks: ScreenShareCallbacks,
    start_control: ScreenShareStartControl,
    reader: RefCell<Option<BrowserVideoFrameReaderHandle>>,
    encoder: RefCell<Option<BrowserVideoEncoder>>,
    audio: RefCell<Option<Rc<BrowserScreenAudio>>>,
    pending_start_cleanup: RefCell<Option<Rc<dyn Fn()>>>,
    stopped: Cell<bool>,
    ended_handler: RefCell<Option<EndedHandler>>,
}

#[derive(Clone, Copy)]
pub(super) enum BrowserCaptureStopReason {
    Manual,
    Chrome,
    SourceEnded,
    StartupCancelled,
    Error,
    Drop,
}

impl BrowserCaptureLifecycle {
    /// Создаёт владельца захвата и подключает системную кнопку завершения Chrome.
    pub(super) fn new(
        stream: MediaStream,
        video_track: MediaStreamTrack,
        callbacks: ScreenShareCallbacks,
    ) -> Rc<Self> {
        let lifecycle = Rc::new(Self {
            stream,
            video_track,
            start_control: callbacks.start_control.clone(),
            callbacks,
            reader: RefCell::new(None),
            encoder: RefCell::new(None),
            audio: RefCell::new(None),
            pending_start_cleanup: RefCell::new(None),
            stopped: Cell::new(false),
            ended_handler: RefCell::new(None),
        });
        lifecycle.install_ended_handler();
        let weak: Weak<Self> = Rc::downgrade(&lifecycle);
        lifecycle.start_control.register_cleanup(Rc::new(move || {
            if let Some(lifecycle) = weak.upgrade() {
                lifecycle.stop(BrowserCaptureStopReason::StartupCancelled);
            }
        }));
        lifecycle
    }

    pub(super) fn set_reader(&self, reader: BrowserVideoFrameReaderHandle) {
        if self.stopped.get() {
            reader.stop();
        } else {
            self.reader.replace(Some(reader));
        }
    }

    pub(super) fn set_encoder(&self, encoder: BrowserVideoEncoder) {
        if self.stopped.get() {
            if let Err(error) = encoder.close() {
                warn!(%error, "failed to close encoder created after capture stopped");
            }
        } else {
            self.encoder.replace(Some(encoder));
        }
    }

    pub(super) fn encoder_handle(&self) -> Option<BrowserVideoEncoderHandle> {
        self.encoder
            .borrow()
            .as_ref()
            .map(BrowserVideoEncoder::handle)
    }

    pub(super) fn take_audio_frames(
        &self,
    ) -> Option<futures_channel::mpsc::Receiver<super::super::backend::EncodedScreenAudioFrame>>
    {
        self.audio.borrow().as_ref()?.take_frames()
    }

    pub(super) fn set_audio(&self, audio: Option<Rc<BrowserScreenAudio>>) {
        if self.stopped.get() {
            if let Some(audio) = audio {
                audio.stop();
            }
        } else {
            self.audio.replace(audio);
        }
    }

    pub(super) fn register_pending_start_cleanup(&self, cleanup: Rc<dyn Fn()>) {
        if self.stopped.get() {
            cleanup();
        } else {
            self.pending_start_cleanup.replace(Some(cleanup));
        }
    }

    pub(super) fn clear_pending_start_cleanup(&self) {
        self.pending_start_cleanup.borrow_mut().take();
    }

    pub(super) fn stop(&self, reason: BrowserCaptureStopReason) {
        if !claim_stop(&self.stopped) {
            debug!(
                reason = stop_reason_name(reason),
                "screen capture cleanup already completed"
            );
            return;
        }
        debug!(
            reason = stop_reason_name(reason),
            stream_active_before = self.stream.active(),
            "stopping browser screen capture"
        );
        self.start_control.cancel();
        if let Some(cleanup) = self.pending_start_cleanup.borrow_mut().take() {
            cleanup();
        }
        if let Some(handler) = self.ended_handler.borrow().as_ref()
            && let Err(error) = self
                .video_track
                .remove_event_listener_with_callback("ended", handler.as_ref().unchecked_ref())
        {
            warn!(error = %js_error_message(error), "failed to remove browser screen capture ended handler");
        }
        self.ended_handler.borrow_mut().take();
        if let Some(audio) = self.audio.borrow_mut().take() {
            audio.stop();
        }
        if let Some(reader) = self.reader.borrow_mut().take() {
            reader.stop();
        }
        if let Some(encoder) = self.encoder.borrow_mut().take() {
            debug!("closing browser screen share VideoEncoder");
            if let Err(error) = encoder.close() {
                warn!(%error, "failed to close browser screen share VideoEncoder");
            } else {
                debug!("closed browser screen share VideoEncoder");
            }
        }
        stop_screen_stream(&self.stream, stop_reason_name(reason));
        debug!(
            reason = stop_reason_name(reason),
            stream_active_after = self.stream.active(),
            "browser screen capture cleanup completed"
        );
        if matches!(
            reason,
            BrowserCaptureStopReason::Manual
                | BrowserCaptureStopReason::Chrome
                | BrowserCaptureStopReason::SourceEnded
                | BrowserCaptureStopReason::Drop
        ) {
            (self.callbacks.on_ended)();
        }
    }

    pub(super) fn is_stopped(&self) -> bool {
        self.stopped.get()
    }

    fn install_ended_handler(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        let ended = Closure::wrap(Box::new(move |_: Event| {
            if let Some(lifecycle) = weak.upgrade() {
                lifecycle.stop(BrowserCaptureStopReason::Chrome);
            }
        }) as Box<dyn FnMut(Event)>);
        if let Err(error) = self
            .video_track
            .add_event_listener_with_callback("ended", ended.as_ref().unchecked_ref())
        {
            warn!(error = %js_error_message(error), "failed to observe browser screen capture ended event");
        }
        self.ended_handler.replace(Some(ended));
    }
}

fn claim_stop(stopped: &Cell<bool>) -> bool {
    !stopped.replace(true)
}

impl Drop for BrowserCaptureLifecycle {
    fn drop(&mut self) {
        self.stop(BrowserCaptureStopReason::Drop);
    }
}

fn stop_reason_name(reason: BrowserCaptureStopReason) -> &'static str {
    match reason {
        BrowserCaptureStopReason::Manual => "manual",
        BrowserCaptureStopReason::Chrome => "chrome",
        BrowserCaptureStopReason::SourceEnded => "source-ended",
        BrowserCaptureStopReason::StartupCancelled => "startup-cancelled",
        BrowserCaptureStopReason::Error => "error",
        BrowserCaptureStopReason::Drop => "drop",
    }
}

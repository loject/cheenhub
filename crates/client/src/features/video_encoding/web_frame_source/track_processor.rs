//! Отмена чтения и освобождение lock у `MediaStreamTrackProcessor`.

use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::{debug, warn};
use js_sys::Promise;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::JsValue;
use wasm_bindgen_futures::{JsFuture, spawn_local};

use super::super::backend::VideoEncodingError;
use super::super::web::BrowserVideoFrame;
use super::{StreamRead, call_method0, read_stream_chunk, video_error};

#[cfg(test)]
mod tests;

pub(super) struct TrackProcessorReaderState {
    reader: JsValue,
    cancelled: Cell<bool>,
    pending_read: Cell<bool>,
    release_requested: Cell<bool>,
    release_attempted: Cell<bool>,
    released: Cell<bool>,
}

impl TrackProcessorReaderState {
    pub(super) fn new(reader: JsValue) -> Self {
        Self {
            reader,
            cancelled: Cell::new(false),
            pending_read: Cell::new(false),
            release_requested: Cell::new(false),
            release_attempted: Cell::new(false),
            released: Cell::new(false),
        }
    }

    pub(super) async fn read(self: &Rc<Self>) -> Result<StreamRead, VideoEncodingError> {
        if self.cancelled.get() {
            return Ok(done_read());
        }
        self.pending_read.set(true);
        let _pending_read = PendingRead::new(self.clone());
        let read = match read_stream_chunk(&self.reader).await {
            Ok(read) => read,
            Err(_) if self.cancelled.get() => return Ok(done_read()),
            Err(error) => return Err(error),
        };
        let cancelled = self.cancelled.get();
        let value = (!read.value.is_null() && !read.value.is_undefined()).then_some(read.value);
        let disposition = route_read_result(cancelled, read.done, value, |value| {
            if !BrowserVideoFrame::close_js_value(&value) {
                warn!("cancelled browser reader returned a non-VideoFrame value");
            }
        });
        match disposition {
            ReadDisposition::Done => {
                if read.done {
                    self.release_requested.set(true);
                    self.release_lock();
                }
                Ok(done_read())
            }
            ReadDisposition::Frame(value) => Ok(StreamRead { done: false, value }),
        }
    }

    pub(super) fn stop(self: &Rc<Self>) {
        if !mark_cancelled(&self.cancelled) {
            return;
        }
        debug!("cancelling browser track-processor reader");
        let cancel = call_method0(&self.reader, "cancel");
        let state = self.clone();
        match cancel {
            Ok(promise) => {
                let Ok(promise) = promise.dyn_into::<Promise>() else {
                    warn!("ReadableStream reader.cancel did not return a Promise");
                    state.release_requested.set(true);
                    if !state.pending_read.get() {
                        state.release_lock();
                    }
                    return;
                };
                spawn_local(async move {
                    match JsFuture::from(promise).await {
                        Ok(_) => debug!("browser track-processor reader cancellation completed"),
                        Err(error) => {
                            warn!(error = %video_error(error), "browser track-processor reader cancellation failed")
                        }
                    }
                    // Lock освобождается только guard'ом после завершения или отмены read.
                    state.release_requested.set(true);
                    state.release_lock();
                });
            }
            Err(error) => {
                warn!(%error, "browser track-processor reader.cancel threw an error");
                state.release_requested.set(true);
                if !state.pending_read.get() {
                    state.release_lock();
                }
            }
        }
    }

    fn release_lock(&self) {
        match release_lock_once(
            self.pending_read.get(),
            self.release_requested.get(),
            &self.release_attempted,
            &self.released,
            || call_method0(&self.reader, "releaseLock").map(|_| ()),
        ) {
            Ok(true) => debug!("released browser track-processor reader lock"),
            Ok(false) => {}
            Err(error) => {
                warn!(%error, "failed to release browser track-processor reader lock; no retry will be attempted");
            }
        }
    }
}

fn done_read() -> StreamRead {
    StreamRead {
        done: true,
        value: JsValue::UNDEFINED,
    }
}

enum ReadDisposition<T> {
    Frame(T),
    Done,
}

fn route_read_result<T>(
    cancelled: bool,
    done: bool,
    value: Option<T>,
    close_discarded: impl FnOnce(T),
) -> ReadDisposition<T> {
    if done {
        return ReadDisposition::Done;
    }
    let Some(value) = value else {
        return ReadDisposition::Done;
    };
    if cancelled {
        close_discarded(value);
        ReadDisposition::Done
    } else {
        ReadDisposition::Frame(value)
    }
}

fn mark_cancelled(cancelled: &Cell<bool>) -> bool {
    !cancelled.replace(true)
}

/// Делает одну попытку освобождения lock и сохраняет результат фактического вызова.
/// После JS-исключения повтор не выполняется: его причина не меняется между вызовами.
fn release_lock_once<E>(
    pending_read: bool,
    release_requested: bool,
    release_attempted: &Cell<bool>,
    released: &Cell<bool>,
    release: impl FnOnce() -> Result<(), E>,
) -> Result<bool, E> {
    if pending_read || !release_requested || released.get() || release_attempted.replace(true) {
        return Ok(false);
    }
    release()?;
    released.set(true);
    Ok(true)
}

struct PendingRead {
    state: Rc<TrackProcessorReaderState>,
}

impl PendingRead {
    fn new(state: Rc<TrackProcessorReaderState>) -> Self {
        Self { state }
    }
}

impl Drop for PendingRead {
    fn drop(&mut self) {
        self.state.pending_read.set(false);
        if self.state.release_requested.get() {
            self.state.release_lock();
        }
    }
}

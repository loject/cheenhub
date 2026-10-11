//! Подтверждения живых команд DSP от Dedicated Worker.

use std::cell::RefCell;
use std::rc::Rc;

use futures_channel::oneshot;
use js_sys::Object;
use wasm_bindgen::prelude::*;
use web_sys::Worker;

use super::{microphone_error, set_property};
use crate::features::microphone::backend::MicrophoneError;

type PendingDenoise = Option<(u64, oneshot::Sender<Result<(), MicrophoneError>>)>;

/// Один ожидающий запрос: provider сериализует смену денойзера этой сессии.
#[derive(Clone, Default)]
pub(super) struct DenoiseControl(Rc<RefCell<PendingDenoise>>);

impl DenoiseControl {
    /// Отправляет режим и ожидает подтверждения того же поколения от worker.
    pub(super) async fn set(
        &self,
        worker: &Worker,
        enabled: bool,
        revision: u64,
    ) -> Result<(), MicrophoneError> {
        let (sender, receiver) = oneshot::channel();
        *self.0.borrow_mut() = Some((revision, sender));
        let message = Object::new();
        set_property(&message, "kind", JsValue::from_str("set-denoise"))?;
        set_property(&message, "denoiseEnabled", JsValue::from_bool(enabled))?;
        set_property(
            &message,
            "denoiseRevision",
            JsValue::from_f64(revision as f64),
        )?;
        worker
            .post_message(message.as_ref())
            .map_err(microphone_error)?;
        receiver.await.map_err(|_| {
            MicrophoneError::new("Worker микрофона завершился во время изменения подавления шума.")
        })?
    }

    /// Завершает только актуальный запрос; запоздалые ответы предыдущего поколения игнорируются.
    pub(super) fn complete(&self, revision: u64, result: Result<(), MicrophoneError>) {
        let mut pending = self.0.borrow_mut();
        if pending
            .as_ref()
            .is_some_and(|(expected, _)| *expected == revision)
            && let Some((_, sender)) = pending.take()
        {
            let _ = sender.send(result);
        }
    }
}

#[cfg(test)]
mod tests;

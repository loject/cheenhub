//! Вспомогательные функции runtime-провайдера демонстрации экрана.

use std::rc::Rc;

use dioxus::core::{Runtime, ScopeId};
use dioxus::prelude::*;
use futures_channel::mpsc;
use futures_util::StreamExt;

use super::backend::{
    EncodedScreenShareFrame, ScreenShareCallbacks, ScreenShareError, ScreenShareFrameCallback,
    ScreenShareSession, ScreenShareStatus,
};

pub(super) fn screen_share_callbacks(
    on_frame: ScreenShareFrameCallback,
    session: Signal<Option<Rc<dyn ScreenShareSession>>>,
    status: Signal<ScreenShareStatus>,
    generation: Signal<u64>,
    start_generation: u64,
    provider_scope: ScopeId,
) -> ScreenShareCallbacks {
    let (events, receiver) = mpsc::unbounded();
    spawn_screen_share_callback_relay(
        receiver,
        on_frame,
        session,
        status,
        generation,
        start_generation,
        provider_scope,
    );

    let ended_events = events.clone();
    let error_events = events.clone();
    ScreenShareCallbacks {
        on_frame: Rc::new(move |frame| {
            let _ = events.unbounded_send(ScreenShareCallbackEvent::Frame(frame));
        }),
        on_ended: Rc::new(move || {
            let _ = ended_events.unbounded_send(ScreenShareCallbackEvent::Ended);
        }),
        on_error: Rc::new(move |error| {
            let _ = error_events.unbounded_send(ScreenShareCallbackEvent::Error(error));
        }),
    }
}

fn spawn_screen_share_callback_relay(
    mut receiver: mpsc::UnboundedReceiver<ScreenShareCallbackEvent>,
    on_frame: ScreenShareFrameCallback,
    mut session: Signal<Option<Rc<dyn ScreenShareSession>>>,
    mut status: Signal<ScreenShareStatus>,
    mut generation: Signal<u64>,
    start_generation: u64,
    provider_scope: ScopeId,
) {
    Runtime::current().in_scope(provider_scope, || {
        spawn(async move {
            while let Some(event) = receiver.next().await {
                match event {
                    ScreenShareCallbackEvent::Frame(frame) => {
                        if generation() == start_generation {
                            on_frame(frame);
                        } else {
                            debug!("ignored stale screen sharing frame callback");
                        }
                    }
                    ScreenShareCallbackEvent::Ended => {
                        if generation() != start_generation {
                            debug!("ignored stale screen sharing ended callback");
                            continue;
                        }
                        let ended_generation_value = next_generation(&mut generation);
                        session.set(None);
                        status.set(ScreenShareStatus::Idle);
                        info!(
                            generation = ended_generation_value,
                            "screen sharing capture ended by source"
                        );
                    }
                    ScreenShareCallbackEvent::Error(error) => {
                        if generation() != start_generation {
                            debug!("ignored stale screen sharing error callback");
                            continue;
                        }
                        let error_generation_value = next_generation(&mut generation);
                        session.set(None);
                        status.set(status_from_runtime_error(error.clone()));
                        warn!(
                            %error,
                            generation = error_generation_value,
                            "screen sharing capture failed at runtime"
                        );
                    }
                }
            }
            debug!("screen sharing callback relay stopped");
        })
    });
}

enum ScreenShareCallbackEvent {
    Frame(EncodedScreenShareFrame),
    Ended,
    Error(ScreenShareError),
}

pub(super) fn status_from_error(error: ScreenShareError) -> ScreenShareStatus {
    if error.is_permission_denied() {
        ScreenShareStatus::PermissionDenied
    } else {
        ScreenShareStatus::Error(error.to_string())
    }
}

pub(super) fn status_from_runtime_error(error: ScreenShareError) -> ScreenShareStatus {
    ScreenShareStatus::Error(error.to_string())
}

pub(super) fn next_generation(generation: &mut Signal<u64>) -> u64 {
    let next_generation = generation.peek().saturating_add(1);
    generation.set(next_generation);
    next_generation
}

#[cfg(test)]
mod tests {
    use super::status_from_runtime_error;
    use crate::features::screen_share::{ScreenShareError, ScreenShareStatus};

    #[test]
    fn runtime_error_always_enters_error_status() {
        assert_eq!(
            status_from_runtime_error(ScreenShareError::new("WGC failure")),
            ScreenShareStatus::Error("WGC failure".to_owned())
        );
    }
}

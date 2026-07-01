//! Вспомогательные функции runtime-провайдера демонстрации экрана.

use std::rc::Rc;

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
) -> ScreenShareCallbacks {
    let (events, receiver) = mpsc::unbounded();
    spawn_screen_share_callback_relay(receiver, on_frame, session, status, generation);

    let ended_events = events.clone();
    ScreenShareCallbacks {
        on_frame: Rc::new(move |frame| {
            let _ = events.unbounded_send(ScreenShareCallbackEvent::Frame(frame));
        }),
        on_ended: Rc::new(move || {
            let _ = ended_events.unbounded_send(ScreenShareCallbackEvent::Ended);
        }),
    }
}

fn spawn_screen_share_callback_relay(
    mut receiver: mpsc::UnboundedReceiver<ScreenShareCallbackEvent>,
    on_frame: ScreenShareFrameCallback,
    mut session: Signal<Option<Rc<dyn ScreenShareSession>>>,
    mut status: Signal<ScreenShareStatus>,
    mut generation: Signal<u64>,
) {
    spawn(async move {
        while let Some(event) = receiver.next().await {
            match event {
                ScreenShareCallbackEvent::Frame(frame) => on_frame(frame),
                ScreenShareCallbackEvent::Ended => {
                    let ended_generation_value = next_generation(&mut generation);
                    session.set(None);
                    status.set(ScreenShareStatus::Idle);
                    info!(
                        generation = ended_generation_value,
                        "screen sharing capture ended by browser"
                    );
                }
            }
        }
        debug!("screen sharing callback relay stopped");
    });
}

enum ScreenShareCallbackEvent {
    Frame(EncodedScreenShareFrame),
    Ended,
}

pub(super) fn status_from_error(error: ScreenShareError) -> ScreenShareStatus {
    if error.is_permission_denied() {
        ScreenShareStatus::PermissionDenied
    } else {
        ScreenShareStatus::Error(error.to_string())
    }
}

pub(super) fn next_generation(generation: &mut Signal<u64>) -> u64 {
    let next_generation = generation.peek().saturating_add(1);
    generation.set(next_generation);
    next_generation
}

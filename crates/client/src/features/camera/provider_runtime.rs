//! Вспомогательные функции runtime-провайдера камеры.

use std::rc::Rc;

use dioxus::prelude::*;
use futures_channel::mpsc;
use futures_util::StreamExt;

use super::backend::{
    CameraCallbacks, CameraError, CameraFrameCallback, CameraSession, CameraStatus,
    EncodedCameraFrame,
};

pub(super) fn camera_callbacks(
    on_frame: CameraFrameCallback,
    session: Signal<Option<Rc<dyn CameraSession>>>,
    status: Signal<CameraStatus>,
    generation: Signal<u64>,
) -> CameraCallbacks {
    let (events, receiver) = mpsc::unbounded();
    spawn_camera_callback_relay(receiver, on_frame, session, status, generation);

    let ended_events = events.clone();
    CameraCallbacks {
        on_frame: Rc::new(move |frame| {
            let _ = events.unbounded_send(CameraCallbackEvent::Frame(frame));
        }),
        on_ended: Rc::new(move || {
            let _ = ended_events.unbounded_send(CameraCallbackEvent::Ended);
        }),
    }
}

fn spawn_camera_callback_relay(
    mut receiver: mpsc::UnboundedReceiver<CameraCallbackEvent>,
    on_frame: CameraFrameCallback,
    mut session: Signal<Option<Rc<dyn CameraSession>>>,
    mut status: Signal<CameraStatus>,
    mut generation: Signal<u64>,
) {
    spawn(async move {
        while let Some(event) = receiver.next().await {
            match event {
                CameraCallbackEvent::Frame(frame) => on_frame(frame),
                CameraCallbackEvent::Ended => {
                    let ended_generation_value = next_generation(&mut generation);
                    session.set(None);
                    status.set(CameraStatus::Idle);
                    info!(
                        generation = ended_generation_value,
                        "camera capture ended by browser"
                    );
                }
            }
        }
        debug!("camera callback relay stopped");
    });
}

enum CameraCallbackEvent {
    Frame(EncodedCameraFrame),
    Ended,
}

pub(super) fn status_from_error(error: CameraError) -> CameraStatus {
    if error.is_permission_denied() {
        CameraStatus::PermissionDenied
    } else {
        CameraStatus::Error(error.to_string())
    }
}

pub(super) fn next_generation(generation: &mut Signal<u64>) -> u64 {
    let next_generation = generation.peek().saturating_add(1);
    generation.set(next_generation);
    next_generation
}

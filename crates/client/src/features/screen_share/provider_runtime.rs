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

/// Сигналы и владелец одной попытки запуска демонстрации.
pub(super) struct CaptureRuntime {
    /// Активная сессия для завершения при событии захвата.
    pub(super) session: Signal<Option<Rc<dyn ScreenShareSession>>>,
    /// Состояние видеодемонстрации.
    pub(super) status: Signal<ScreenShareStatus>,
    /// Текущее поколение владельца захвата.
    pub(super) generation: Signal<u64>,
    /// Поколение, которому принадлежат callback.
    pub(super) start_generation: u64,
    /// Scope, переживающий управляющие кнопки.
    pub(super) provider_scope: ScopeId,
    /// Состояние системного аудио текущего захвата.
    pub(super) audio_state: Signal<super::backend::ScreenShareAudioState>,
    /// Продуктовые сообщения об ограничениях или сбое звука.
    pub(super) on_warning: Rc<dyn Fn(&str)>,
}

pub(super) fn screen_share_callbacks(
    on_frame: ScreenShareFrameCallback,
    runtime: CaptureRuntime,
) -> ScreenShareCallbacks {
    let (events, receiver) = mpsc::unbounded();
    spawn_screen_share_callback_relay(receiver, on_frame, runtime);

    let ended_events = events.clone();
    let error_events = events.clone();
    let audio_events = events.clone();
    ScreenShareCallbacks {
        on_audio_state: Rc::new(move |state| {
            let _ = audio_events.unbounded_send(ScreenShareCallbackEvent::Audio(state));
        }),
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
    runtime: CaptureRuntime,
) {
    let CaptureRuntime {
        mut session,
        mut status,
        mut generation,
        start_generation,
        provider_scope,
        mut audio_state,
        on_warning,
    } = runtime;
    Runtime::current().in_scope(provider_scope, || {
        spawn(async move {
            while let Some(event) = receiver.next().await {
                match event {
                    ScreenShareCallbackEvent::Audio(next) => {
                        if generation() != start_generation { continue; }
                        match &next {
                            super::backend::ScreenShareAudioState::Live { own_audio_restricted: false, .. } =>
                                on_warning("В системный звук могут попасть голоса собеседников."),
                            super::backend::ScreenShareAudioState::Unavailable =>
                                on_warning("Не удалось включить звук демонстрации. Экран передаётся без звука."),
                            _ => {}
                        }
                        audio_state.set(next);
                    }
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
                        audio_state.set(super::backend::ScreenShareAudioState::Off);
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
                        audio_state.set(super::backend::ScreenShareAudioState::Off);
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
    Audio(super::backend::ScreenShareAudioState),
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
mod tests;

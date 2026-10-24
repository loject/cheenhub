//! Последовательная публикация явного выбора микрофона основной voice-сессией.

use cheenhub_contracts::realtime::{
    RealtimeKind, RealtimeModule, SetVoiceMicrophoneState, VoiceChatKind, VoiceRoomSnapshot,
};
use dioxus::prelude::*;
use futures_channel::mpsc;
use futures_util::{
    FutureExt, StreamExt,
    future::{Either, select},
};
use std::{cell::Cell, rc::Rc};

use super::state::{VoiceConnectionState, VoiceRoomTarget};
use crate::features::{
    microphone::MicrophoneHandle,
    realtime::{RealtimeError, RealtimeHandle},
    runtime::sleep_ms,
};

#[derive(Clone, PartialEq, Eq)]
struct Publication {
    target: VoiceRoomTarget,
    generation: u64,
    enabled: bool,
}

/// Публикует выбор при изменении и повторяет неуспешную попытку до смены цели.
///
/// Один coroutine сохраняет порядок быстрых mute/unmute. Ответ не заменяет список
/// участников: он может быть старее уже полученного ParticipantsChanged.
pub(super) fn use_microphone_state_publication(
    state: Signal<VoiceConnectionState>,
    generation: Rc<Cell<u64>>,
    microphone: MicrophoneHandle,
    realtime: RealtimeHandle,
) {
    let observed_microphone = microphone.clone();
    let publisher = use_coroutine(move |mut wake: mpsc::UnboundedReceiver<()>| {
        let generation = generation.clone();
        let microphone = microphone.clone();
        let realtime = realtime.clone();
        async move {
            let mut acknowledged = None::<Publication>;
            let mut failed = false;
            while wake.next().await.is_some() {
                loop {
                    while wake.try_recv().is_ok() {}
                    let VoiceConnectionState::Connected { target, .. } = state.peek().clone()
                    else {
                        acknowledged = None;
                        failed = false;
                        break;
                    };
                    let publication = Publication {
                        target,
                        generation: generation.get(),
                        enabled: microphone.voice_enabled_untracked(),
                    };
                    if acknowledged.as_ref() == Some(&publication) {
                        break;
                    }
                    let result = publish(&realtime, &publication).await;
                    match result {
                        Ok(()) => {
                            debug!(room_id = %publication.target.room_id, microphone_enabled = publication.enabled,
                            "synchronized voice microphone choice");
                            acknowledged = Some(publication);
                            failed = false;
                        }
                        Err(error) => {
                            if !failed {
                                warn!(%error, room_id = %publication.target.room_id,
                                "failed to synchronize microphone choice; retrying while connected");
                                failed = true;
                            }
                            sleep_ms(1_000).await;
                        }
                    }
                }
            }
        }
    });
    use_effect(move || {
        let _ = state();
        let _ = observed_microphone.voice_enabled();
        publisher.send(());
    });
}

async fn publish(
    realtime: &RealtimeHandle,
    publication: &Publication,
) -> Result<(), RealtimeError> {
    let request = realtime.request::<_, VoiceRoomSnapshot>(
        RealtimeModule::VoiceChat,
        RealtimeKind::VoiceChat(VoiceChatKind::SetMicrophoneState),
        SetVoiceMicrophoneState {
            room_id: publication.target.room_id.clone(),
            microphone_enabled: publication.enabled,
        },
    );
    match select(request.boxed_local(), sleep_ms(5_000).boxed_local()).await {
        Either::Left((result, _)) => result.map(|_| ()),
        Either::Right(_) => Err(RealtimeError::new(
            "Не удалось подтвердить состояние микрофона.",
        )),
    }
}

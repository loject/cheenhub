//! Последовательная публикация выбора входящего звука голосовой сессией.

use cheenhub_contracts::realtime::{
    RealtimeKind, RealtimeModule, SetVoiceOutputState, VoiceChatKind, VoiceRoomSnapshot,
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
    audio_playback::AudioPlaybackHandle,
    realtime::{RealtimeError, RealtimeHandle},
    runtime::sleep_ms,
};

#[derive(Clone, PartialEq, Eq)]
struct Publication {
    target: VoiceRoomTarget,
    generation: u64,
    muted: bool,
}

/// Синхронизирует отключение входящего звука и повторяет запрос после сбоя.
///
/// Один coroutine сохраняет порядок быстрых переключений. Ответный снимок
/// не применяется поверх более нового события ParticipantsChanged.
pub(super) fn use_output_state_publication(
    state: Signal<VoiceConnectionState>,
    generation: Rc<Cell<u64>>,
    playback: AudioPlaybackHandle,
    realtime: RealtimeHandle,
) {
    let observed_playback = playback.clone();
    let publisher = use_coroutine(move |mut wake: mpsc::UnboundedReceiver<()>| {
        let generation = generation.clone();
        let playback = playback.clone();
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
                        muted: playback.is_muted(),
                    };
                    if acknowledged.as_ref() == Some(&publication) {
                        break;
                    }
                    match publish(&realtime, &publication).await {
                        Ok(()) => {
                            debug!(room_id = %publication.target.room_id, output_muted = publication.muted,
                                "synchronized voice output mute choice");
                            acknowledged = Some(publication);
                            failed = false;
                        }
                        Err(error) => {
                            if !failed {
                                warn!(%error, room_id = %publication.target.room_id,
                                    "failed to synchronize output mute choice; retrying while connected");
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
        let _ = observed_playback.is_muted();
        publisher.send(());
    });
}

async fn publish(
    realtime: &RealtimeHandle,
    publication: &Publication,
) -> Result<(), RealtimeError> {
    let request = realtime.request::<_, VoiceRoomSnapshot>(
        RealtimeModule::VoiceChat,
        RealtimeKind::VoiceChat(VoiceChatKind::SetOutputState),
        SetVoiceOutputState {
            room_id: publication.target.room_id.clone(),
            output_muted: publication.muted,
        },
    );
    match select(request.boxed_local(), sleep_ms(5_000).boxed_local()).await {
        Either::Left((result, _)) => result.map(|_| ()),
        Either::Right(_) => Err(RealtimeError::new(
            "Не удалось подтвердить состояние входящего звука.",
        )),
    }
}

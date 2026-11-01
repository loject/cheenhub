//! Последовательная публикация состояния микрофона и входящего звука голосовой сессии.

use cheenhub_contracts::realtime::{
    RealtimeKind, RealtimeModule, SetVoiceMicrophoneState, SetVoiceOutputState, VoiceChatKind,
    VoiceRoomSnapshot,
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
    microphone::MicrophoneHandle,
    realtime::{RealtimeError, RealtimeHandle},
    runtime::sleep_ms,
};

#[derive(Clone, PartialEq, Eq)]
struct Publication {
    target: VoiceRoomTarget,
    generation: u64,
    output_muted: bool,
    microphone_enabled: bool,
}

/// Синхронизирует микрофон и входящий звук одним последовательным coroutine.
///
/// При deafen сначала выключает микрофон на сервере, затем входящее воспроизведение.
/// При снятии deafen сначала включает воспроизведение, затем восстанавливает выбор микрофона.
/// Локальный захват останавливается сразу при deafen, не меняя сохранённое предпочтение.
pub(super) fn use_output_state_publication(
    state: Signal<VoiceConnectionState>,
    generation: Rc<Cell<u64>>,
    playback: AudioPlaybackHandle,
    microphone: MicrophoneHandle,
    realtime: RealtimeHandle,
) {
    let observed_playback = playback.clone();
    let observed_microphone = microphone.clone();
    let publisher = use_coroutine(move |mut wake: mpsc::UnboundedReceiver<()>| {
        let generation = generation.clone();
        let playback = playback.clone();
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
                    let output_muted = playback.is_muted();
                    let publication = Publication {
                        target,
                        generation: generation.get(),
                        output_muted,
                        microphone_enabled: microphone.voice_enabled_untracked() && !output_muted,
                    };
                    if acknowledged.as_ref() == Some(&publication) {
                        break;
                    }
                    if output_muted {
                        microphone.stop();
                    }
                    let result = if output_muted {
                        match publish_microphone(&realtime, &publication).await {
                            Ok(()) => publish_output(&realtime, &publication).await,
                            Err(error) => Err(error),
                        }
                    } else {
                        match publish_output(&realtime, &publication).await {
                            Ok(()) => publish_microphone(&realtime, &publication).await,
                            Err(error) => Err(error),
                        }
                    };
                    match result {
                        Ok(()) => {
                            debug!(
                                room_id = %publication.target.room_id,
                                output_muted = publication.output_muted,
                                microphone_enabled = publication.microphone_enabled,
                                "synchronized voice microphone and output state"
                            );
                            acknowledged = Some(publication.clone());
                            failed = false;
                        }
                        Err(error) => {
                            if !failed {
                                warn!(
                                    %error,
                                    room_id = %publication.target.room_id,
                                    "failed to synchronize voice audio state; retrying while connected"
                                );
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
        let _ = observed_microphone.voice_enabled();
        publisher.send(());
    });
}

async fn publish_microphone(
    realtime: &RealtimeHandle,
    publication: &Publication,
) -> Result<(), RealtimeError> {
    let request = realtime.request::<_, VoiceRoomSnapshot>(
        RealtimeModule::VoiceChat,
        RealtimeKind::VoiceChat(VoiceChatKind::SetMicrophoneState),
        SetVoiceMicrophoneState {
            room_id: publication.target.room_id.clone(),
            microphone_enabled: publication.microphone_enabled,
        },
    );
    await_publication(request).await.map(|_| ())
}

async fn publish_output(
    realtime: &RealtimeHandle,
    publication: &Publication,
) -> Result<(), RealtimeError> {
    let request = realtime.request::<_, VoiceRoomSnapshot>(
        RealtimeModule::VoiceChat,
        RealtimeKind::VoiceChat(VoiceChatKind::SetOutputState),
        SetVoiceOutputState {
            room_id: publication.target.room_id.clone(),
            output_muted: publication.output_muted,
        },
    );
    await_publication(request).await.map(|_| ())
}

async fn await_publication<'a, T: 'a>(
    request: impl std::future::Future<Output = Result<T, RealtimeError>> + 'a,
) -> Result<T, RealtimeError> {
    match select(request.boxed_local(), sleep_ms(5_000).boxed_local()).await {
        Either::Left((result, _)) => result,
        Either::Right(_) => Err(RealtimeError::new(
            "Не удалось подтвердить состояние голосового аудио.",
        )),
    }
}

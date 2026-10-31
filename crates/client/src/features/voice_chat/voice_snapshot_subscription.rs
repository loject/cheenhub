//! Подписка на снимки участников с освобождением ушедших медиасессий.

use dioxus::prelude::*;
use futures_util::StreamExt;

use super::realtime;
use super::state::{VoiceConnectionHandle, VoiceConnectionState};
use super::video_streams::ParticipantVideoHandle;
use crate::features::audio_playback::AudioPlaybackHandle;
use crate::features::realtime::RealtimeHandle;

pub(super) fn use_voice_snapshot_subscription(
    realtime: RealtimeHandle,
    handle: VoiceConnectionHandle,
    video: ParticipantVideoHandle,
    playback: AudioPlaybackHandle,
) {
    use_hook(move || {
        spawn(async move {
            let mut snapshots = realtime::subscribe_voice_chat(&realtime);
            while let Some(snapshot) = snapshots.next().await {
                if let VoiceConnectionState::Connected {
                    target,
                    participants,
                } = handle.state()
                    && target.server_id == snapshot.server_id
                    && target.room_id == snapshot.room_id
                {
                    video.reconcile_participants(&participants, &snapshot.participants);
                    super::inbound_audio::reconcile_audio_sessions(
                        &playback,
                        &participants,
                        &snapshot.participants,
                    );
                }
                handle.apply_snapshot(snapshot);
            }
        })
    });
}

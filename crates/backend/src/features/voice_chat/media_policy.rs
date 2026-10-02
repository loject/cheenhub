//! Проверка ограничений исходящих видеопубликаций голосовой комнаты.

use std::time::{Duration, Instant};

use cheenhub_contracts::{
    media::{
        MEDIA_DATAGRAM_FLAG_FRAGMENTED, MEDIA_DATAGRAM_FLAG_KEY_FRAME, MediaDatagram,
        MediaDatagramKind,
    },
    video_presets::{VideoPresetId, VideoStreamSource},
};
use uuid::Uuid;

use super::infrastructure::{InMemoryVoicePresenceStore, VoicePresence};
use vp9::parse_key_frame_dimensions;

mod vp9;

const VIDEO_FRAGMENT_HEADER_LEN: usize = 8;
const FPS_MEASUREMENT_WINDOW: Duration = Duration::from_secs(1);
const FPS_BLOCK_DURATION: Duration = Duration::from_secs(1);
const FPS_JITTER_ALLOWANCE: u32 = 2;
const RECENT_SEQUENCE_LIMIT: usize = 128;
/// Окно, в течение которого видеоисточник считается активным после последнего допустимого кадра.
const VIDEO_ACTIVITY_WINDOW: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VideoAdmission {
    Forward,
    Drop(VideoDropReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VideoDropReason {
    MalformedFragment,
    AwaitingFirstFragment,
    AwaitingKeyFrame,
    InvalidVp9KeyFrame,
    UnsupportedResolution { width: u32, height: u32 },
    FpsLimitExceeded { max_fps: u32, observed_frames: u32 },
    FpsBlockActive,
}

#[derive(Default)]
pub(super) struct VideoPublicationTracker {
    publications: Vec<VideoPublication>,
}

impl VideoPublicationTracker {
    pub(super) fn inspect(
        &mut self,
        session_id: Uuid,
        datagram: &MediaDatagram,
        allowed_presets: &[VideoPresetId],
    ) -> VideoAdmission {
        self.inspect_at(session_id, datagram, allowed_presets, Instant::now())
    }

    fn inspect_at(
        &mut self,
        session_id: Uuid,
        datagram: &MediaDatagram,
        allowed_presets: &[VideoPresetId],
        now: Instant,
    ) -> VideoAdmission {
        let key = VideoPublicationKey {
            session_id,
            room_id: datagram.room_id,
            kind: datagram.kind,
        };
        let publication = match self.publications.iter_mut().find(|entry| entry.key == key) {
            Some(publication) => publication,
            None => {
                self.publications.push(VideoPublication::new(key, now));
                self.publications
                    .last_mut()
                    .expect("publication was inserted")
            }
        };

        let fragment = match frame_fragment(datagram) {
            Ok(fragment) => fragment,
            Err(reason) => return VideoAdmission::Drop(reason),
        };
        if !fragment.is_first {
            return publication
                .decision_for(datagram.sequence)
                .unwrap_or(VideoAdmission::Drop(VideoDropReason::AwaitingFirstFragment));
        }
        if let Some(decision) = publication.decision_for(datagram.sequence) {
            return decision;
        }

        let is_key_frame = datagram.flags & MEDIA_DATAGRAM_FLAG_KEY_FRAME != 0;
        let decision = publication.inspect_frame(
            datagram.sequence,
            is_key_frame,
            fragment.vp9_payload,
            allowed_presets,
            now,
        );
        publication.remember(datagram.sequence, decision);
        if decision == VideoAdmission::Forward {
            // Только что прошедший проверку кадр продлевает активность источника;
            // повторные и отклонённые пакеты возвращаются выше без обновления времени.
            publication.last_forward_at = Some(now);
        }
        decision
    }

    pub(super) fn remove_presences(&mut self, removed: &[VoicePresence]) {
        self.publications.retain(|publication| {
            !removed.iter().any(|presence| {
                publication.key.session_id == presence.session_id
                    && publication.key.room_id == presence.room_id
            })
        });
    }

    /// Убирает один источник по явной остановке видеопотока владельцем сессии.
    pub(super) fn remove_source(
        &mut self,
        session_id: Uuid,
        room_id: Uuid,
        kind: MediaDatagramKind,
    ) {
        self.publications.retain(|publication| {
            !(publication.key.session_id == session_id
                && publication.key.room_id == room_id
                && publication.key.kind == kind)
        });
    }

    /// Считает источники, от которых недавно пришёл допустимый кадр.
    pub(super) fn active_source_count(&self, now: Instant) -> usize {
        self.publications
            .iter()
            .filter(|publication| {
                publication
                    .last_forward_at
                    .is_some_and(|last| now.saturating_duration_since(last) < VIDEO_ACTIVITY_WINDOW)
            })
            .count()
    }
}

impl InMemoryVoicePresenceStore {
    pub(super) async fn inspect_video_datagram(
        &self,
        session_id: Uuid,
        datagram: &MediaDatagram,
        allowed_presets: &[VideoPresetId],
    ) -> VideoAdmission {
        self.video_publications
            .lock()
            .await
            .inspect(session_id, datagram, allowed_presets)
    }

    /// Убирает один видеоисточник по явному запросу остановки видеопотока.
    pub(super) async fn remove_video_source(
        &self,
        session_id: Uuid,
        room_id: Uuid,
        kind: MediaDatagramKind,
    ) {
        self.video_publications
            .lock()
            .await
            .remove_source(session_id, room_id, kind);
    }

    /// Возвращает число активных видеоисточников по времени последнего допустимого кадра.
    pub(super) async fn active_video_source_count(&self) -> usize {
        self.video_publications
            .lock()
            .await
            .active_source_count(Instant::now())
    }
}

/// Ключ одного видеоисточника: камера и экран сессии учитываются раздельно.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VideoPublicationKey {
    session_id: Uuid,
    room_id: Uuid,
    kind: MediaDatagramKind,
}

struct VideoPublication {
    key: VideoPublicationKey,
    selected_preset: Option<VideoPresetId>,
    window_started_at: Instant,
    window_frames: u32,
    blocked_until: Option<Instant>,
    recent_decisions: Vec<(u64, VideoAdmission)>,
    last_forward_at: Option<Instant>,
}

impl VideoPublication {
    fn new(key: VideoPublicationKey, now: Instant) -> Self {
        Self {
            key,
            selected_preset: None,
            window_started_at: now,
            window_frames: 0,
            blocked_until: None,
            recent_decisions: Vec::new(),
            last_forward_at: None,
        }
    }

    fn inspect_frame(
        &mut self,
        _sequence: u64,
        is_key_frame: bool,
        vp9_payload: &[u8],
        allowed_presets: &[VideoPresetId],
        now: Instant,
    ) -> VideoAdmission {
        if let Some(blocked_until) = self.blocked_until
            && (now < blocked_until || !is_key_frame)
        {
            return VideoAdmission::Drop(VideoDropReason::FpsBlockActive);
        }

        if is_key_frame {
            let Some((width, height)) = parse_key_frame_dimensions(vp9_payload) else {
                self.block();
                return VideoAdmission::Drop(VideoDropReason::InvalidVp9KeyFrame);
            };
            let Some(preset) = allowed_presets.iter().copied().find(|preset| {
                let spec = preset.spec();
                Some(spec.source) == source_for_kind(self.key.kind)
                    && spec.width == width
                    && spec.height == height
            }) else {
                self.block();
                return VideoAdmission::Drop(VideoDropReason::UnsupportedResolution {
                    width,
                    height,
                });
            };
            if self.selected_preset.is_none() {
                self.selected_preset = Some(preset);
                self.blocked_until = None;
                self.window_started_at = now;
                self.window_frames = 1;
                return VideoAdmission::Forward;
            }
            self.selected_preset = Some(preset);
        }

        let Some(preset) = self.selected_preset else {
            return VideoAdmission::Drop(VideoDropReason::AwaitingKeyFrame);
        };
        self.window_frames = self.window_frames.saturating_add(1);
        let elapsed = now.saturating_duration_since(self.window_started_at);
        if elapsed >= FPS_MEASUREMENT_WINDOW {
            let max_fps = preset.spec().max_fps;
            let allowed_frames = max_fps
                .saturating_mul(elapsed.as_millis().min(u128::from(u32::MAX)) as u32)
                / 1_000
                + FPS_JITTER_ALLOWANCE;
            if self.window_frames > allowed_frames {
                let observed_frames = self.window_frames;
                self.block();
                self.blocked_until = Some(now + FPS_BLOCK_DURATION);
                return VideoAdmission::Drop(VideoDropReason::FpsLimitExceeded {
                    max_fps,
                    observed_frames,
                });
            }
            self.window_started_at = now;
            self.window_frames = 0;
        }
        VideoAdmission::Forward
    }

    fn block(&mut self) {
        self.selected_preset = None;
        self.window_frames = 0;
    }

    fn decision_for(&self, sequence: u64) -> Option<VideoAdmission> {
        self.recent_decisions
            .iter()
            .rev()
            .find_map(|(candidate, decision)| (*candidate == sequence).then_some(*decision))
    }

    fn remember(&mut self, sequence: u64, decision: VideoAdmission) {
        if self.recent_decisions.len() == RECENT_SEQUENCE_LIMIT {
            self.recent_decisions.remove(0);
        }
        self.recent_decisions.push((sequence, decision));
    }
}

struct FrameFragment<'a> {
    is_first: bool,
    vp9_payload: &'a [u8],
}

fn frame_fragment(datagram: &MediaDatagram) -> Result<FrameFragment<'_>, VideoDropReason> {
    if datagram.flags & MEDIA_DATAGRAM_FLAG_FRAGMENTED == 0 {
        return Ok(FrameFragment {
            is_first: true,
            vp9_payload: &datagram.payload,
        });
    }
    if datagram.payload.len() < VIDEO_FRAGMENT_HEADER_LEN {
        return Err(VideoDropReason::MalformedFragment);
    }
    let fragment_index = u16::from_be_bytes([datagram.payload[4], datagram.payload[5]]);
    let fragment_count = u16::from_be_bytes([datagram.payload[6], datagram.payload[7]]);
    if fragment_count == 0 || fragment_index >= fragment_count {
        return Err(VideoDropReason::MalformedFragment);
    }
    Ok(FrameFragment {
        is_first: fragment_index == 0,
        vp9_payload: &datagram.payload[VIDEO_FRAGMENT_HEADER_LEN..],
    })
}

fn source_for_kind(kind: MediaDatagramKind) -> Option<VideoStreamSource> {
    match kind {
        MediaDatagramKind::CameraFrame => Some(VideoStreamSource::Camera),
        MediaDatagramKind::ScreenFrame => Some(VideoStreamSource::ScreenShare),
        MediaDatagramKind::VoiceFrame => None,
    }
}

#[cfg(test)]
mod activity_tests;
#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;

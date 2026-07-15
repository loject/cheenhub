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
        user_id: Uuid,
        datagram: &MediaDatagram,
        allowed_presets: &[VideoPresetId],
    ) -> VideoAdmission {
        self.inspect_at(
            session_id,
            user_id,
            datagram,
            allowed_presets,
            Instant::now(),
        )
    }

    fn inspect_at(
        &mut self,
        session_id: Uuid,
        user_id: Uuid,
        datagram: &MediaDatagram,
        allowed_presets: &[VideoPresetId],
        now: Instant,
    ) -> VideoAdmission {
        let key = VideoPublicationKey {
            session_id,
            user_id,
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
        decision
    }

    pub(super) fn remove_presences(&mut self, removed: &[VoicePresence]) {
        self.publications.retain(|publication| {
            !removed.iter().any(|presence| {
                publication.key.session_id == presence.session_id
                    && publication.key.user_id == presence.user_id
                    && publication.key.room_id == presence.room_id
            })
        });
    }
}

impl InMemoryVoicePresenceStore {
    pub(super) async fn inspect_video_datagram(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        datagram: &MediaDatagram,
        allowed_presets: &[VideoPresetId],
    ) -> VideoAdmission {
        self.video_publications
            .lock()
            .await
            .inspect(session_id, user_id, datagram, allowed_presets)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VideoPublicationKey {
    session_id: Uuid,
    user_id: Uuid,
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
mod tests {
    use super::*;
    use cheenhub_contracts::{
        media::{MediaCodec, MediaDatagram},
        video_presets::{BASE_CAMERA_VIDEO_PRESETS, BASE_SCREEN_SHARE_VIDEO_PRESETS},
    };

    #[test]
    fn screen_policy_accepts_both_base_resolutions() {
        for (sequence, width, height) in [(1, 1280, 720), (2, 1920, 1080)] {
            let mut tracker = VideoPublicationTracker::default();
            let mut datagram = video_datagram(sequence, true, width, height);
            datagram.kind = MediaDatagramKind::ScreenFrame;
            assert_eq!(
                tracker.inspect_at(
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                    &datagram,
                    BASE_SCREEN_SHARE_VIDEO_PRESETS,
                    Instant::now(),
                ),
                VideoAdmission::Forward
            );
        }
    }

    #[test]
    fn camera_policy_rejects_1080p() {
        let mut tracker = VideoPublicationTracker::default();
        let datagram = video_datagram(1, true, 1920, 1080);
        assert_eq!(
            tracker.inspect_at(
                Uuid::new_v4(),
                Uuid::new_v4(),
                &datagram,
                BASE_CAMERA_VIDEO_PRESETS,
                Instant::now(),
            ),
            VideoAdmission::Drop(VideoDropReason::UnsupportedResolution {
                width: 1920,
                height: 1080,
            })
        );
    }

    #[test]
    fn fragmented_frame_is_counted_once() {
        let mut tracker = VideoPublicationTracker::default();
        let session_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let now = Instant::now();
        let first = fragmented(video_datagram(1, true, 1280, 720), 0, 2);
        let mut second = fragmented(video_datagram(1, true, 1280, 720), 1, 2);
        second.room_id = first.room_id;
        assert_eq!(
            tracker.inspect_at(session_id, user_id, &first, BASE_CAMERA_VIDEO_PRESETS, now),
            VideoAdmission::Forward
        );
        assert_eq!(
            tracker.inspect_at(session_id, user_id, &second, BASE_CAMERA_VIDEO_PRESETS, now),
            VideoAdmission::Forward
        );
        assert_eq!(tracker.publications[0].window_frames, 1);
    }

    #[test]
    fn sustained_fps_violation_blocks_until_later_key_frame() {
        let mut tracker = VideoPublicationTracker::default();
        let session_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let started = Instant::now();
        let key = video_datagram(1, true, 1280, 720);
        let room_id = key.room_id;
        assert_eq!(
            tracker.inspect_at(
                session_id,
                user_id,
                &key,
                BASE_CAMERA_VIDEO_PRESETS,
                started
            ),
            VideoAdmission::Forward
        );
        for sequence in 2..=26 {
            let mut frame = video_datagram(sequence, false, 0, 0);
            frame.room_id = room_id;
            assert_eq!(
                tracker.inspect_at(
                    session_id,
                    user_id,
                    &frame,
                    BASE_CAMERA_VIDEO_PRESETS,
                    started + Duration::from_millis(sequence * 30),
                ),
                VideoAdmission::Forward
            );
        }
        let mut violating = video_datagram(27, false, 0, 0);
        violating.room_id = room_id;
        assert!(matches!(
            tracker.inspect_at(
                session_id,
                user_id,
                &violating,
                BASE_CAMERA_VIDEO_PRESETS,
                started + Duration::from_millis(1_010),
            ),
            VideoAdmission::Drop(VideoDropReason::FpsLimitExceeded { .. })
        ));
        let mut early_key = video_datagram(28, true, 1280, 720);
        early_key.room_id = room_id;
        assert_eq!(
            tracker.inspect_at(
                session_id,
                user_id,
                &early_key,
                BASE_CAMERA_VIDEO_PRESETS,
                started + Duration::from_millis(1_500),
            ),
            VideoAdmission::Drop(VideoDropReason::FpsBlockActive)
        );
        let mut later_key = video_datagram(29, true, 1280, 720);
        later_key.room_id = room_id;
        assert_eq!(
            tracker.inspect_at(
                session_id,
                user_id,
                &later_key,
                BASE_CAMERA_VIDEO_PRESETS,
                started + Duration::from_millis(2_100),
            ),
            VideoAdmission::Forward
        );
    }

    fn video_datagram(sequence: u64, key_frame: bool, width: u32, height: u32) -> MediaDatagram {
        MediaDatagram {
            kind: MediaDatagramKind::CameraFrame,
            codec: MediaCodec::Vp9,
            flags: if key_frame {
                MEDIA_DATAGRAM_FLAG_KEY_FRAME
            } else {
                0
            },
            sequence,
            timestamp_us: 0,
            duration_us: 0,
            room_id: Uuid::new_v4(),
            sender_user_id: Uuid::nil(),
            payload: if key_frame {
                vp9_key_frame(width, height)
            } else {
                Vec::new()
            },
        }
    }

    fn fragmented(mut datagram: MediaDatagram, index: u16, count: u16) -> MediaDatagram {
        let bytes = std::mem::take(&mut datagram.payload);
        datagram.flags |= MEDIA_DATAGRAM_FLAG_FRAGMENTED;
        datagram.payload = Vec::new();
        datagram
            .payload
            .extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        datagram.payload.extend_from_slice(&index.to_be_bytes());
        datagram.payload.extend_from_slice(&count.to_be_bytes());
        datagram.payload.extend_from_slice(&bytes);
        datagram
    }

    fn vp9_key_frame(width: u32, height: u32) -> Vec<u8> {
        let mut writer = BitWriter::default();
        writer.write(0b10, 2);
        writer.write(0, 1);
        writer.write(0, 1);
        writer.write(0, 1);
        writer.write(0, 1);
        writer.write(1, 1);
        writer.write(0, 1);
        writer.write(0x49_83_42, 24);
        writer.write(1, 3);
        writer.write(0, 1);
        writer.write(width - 1, 16);
        writer.write(height - 1, 16);
        writer.bytes
    }

    #[derive(Default)]
    struct BitWriter {
        bytes: Vec<u8>,
        bit_offset: usize,
    }

    impl BitWriter {
        fn write(&mut self, value: u32, count: usize) {
            for bit_index in (0..count).rev() {
                if self.bit_offset.is_multiple_of(8) {
                    self.bytes.push(0);
                }
                let bit = ((value >> bit_index) & 1) as u8;
                let byte_index = self.bit_offset / 8;
                let shift = 7 - self.bit_offset % 8;
                self.bytes[byte_index] |= bit << shift;
                self.bit_offset += 1;
            }
        }
    }
}

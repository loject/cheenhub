//! Проверка ограничений исходящих видеопубликаций голосовой комнаты.

use std::time::{Duration, Instant};

use cheenhub_contracts::{
    media::{MEDIA_DATAGRAM_FLAG_KEY_FRAME, MediaDatagram, MediaDatagramKind},
    video_presets::{VideoBitrateLimit, VideoStreamSource},
};
use uuid::Uuid;

use super::infrastructure::{InMemoryVoicePresenceStore, VoicePresence};
use super::media::MediaDatagramHeader;
use vp9::parse_key_frame_dimensions;

mod fragment;
mod vp9;
use fragment::{FragmentState, frame_fragment};

/// Максимальный накопленный burst видеотрафика в секундах целевого битрейта.
const VIDEO_BURST_SECONDS: u64 = 2;
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
    /// Фрагмент или целый кадр уже были пересланы либо вышли из окна последовательностей.
    DuplicateFrame,
    InvalidVp9KeyFrame,
    SourceNotAllowed,
    BitrateLimitExceeded {
        max_bitrate_bps: u32,
        frame_bytes: usize,
    },
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
        bitrate_limit: VideoBitrateLimit,
    ) -> VideoAdmission {
        self.inspect_at(session_id, datagram, bitrate_limit, Instant::now())
    }

    fn inspect_at(
        &mut self,
        session_id: Uuid,
        datagram: &MediaDatagram,
        bitrate_limit: VideoBitrateLimit,
        now: Instant,
    ) -> VideoAdmission {
        self.inspect_frame_at(
            VideoFrameView {
                session_id,
                kind: datagram.kind,
                flags: datagram.flags,
                sequence: datagram.sequence,
                room_id: datagram.room_id,
                payload: &datagram.payload,
            },
            bitrate_limit,
            now,
        )
    }

    fn inspect_header_at(
        &mut self,
        session_id: Uuid,
        header: &MediaDatagramHeader,
        payload: &[u8],
        bitrate_limit: VideoBitrateLimit,
        now: Instant,
    ) -> VideoAdmission {
        self.inspect_frame_at(
            VideoFrameView {
                session_id,
                kind: header.kind,
                flags: header.flags,
                sequence: header.sequence,
                room_id: header.room_id,
                payload,
            },
            bitrate_limit,
            now,
        )
    }

    fn inspect_frame_at(
        &mut self,
        frame: VideoFrameView<'_>,
        bitrate_limit: VideoBitrateLimit,
        now: Instant,
    ) -> VideoAdmission {
        let key = VideoPublicationKey {
            session_id: frame.session_id,
            room_id: frame.room_id,
            kind: frame.kind,
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

        if publication.highest_sequence.is_some_and(|highest| {
            highest.saturating_sub(frame.sequence) >= RECENT_SEQUENCE_LIMIT as u64
        }) {
            return VideoAdmission::Drop(VideoDropReason::DuplicateFrame);
        }

        let fragment = match frame_fragment(frame.flags, frame.payload) {
            Ok(fragment) => fragment,
            Err(reason) => {
                if publication.decision_for(frame.sequence) == Some(VideoAdmission::Forward) {
                    publication.has_key_frame = false;
                    publication.reject_sequence(frame.sequence, reason);
                }
                return VideoAdmission::Drop(reason);
            }
        };
        if fragment.index != 0 {
            let Some(decision) = publication.decision_for(frame.sequence) else {
                return VideoAdmission::Drop(VideoDropReason::AwaitingFirstFragment);
            };
            if decision != VideoAdmission::Forward {
                return decision;
            }
            let Some((_, state)) = publication
                .fragment_bytes_seen
                .iter_mut()
                .find(|(sequence, _)| *sequence == frame.sequence)
            else {
                return VideoAdmission::Drop(VideoDropReason::AwaitingFirstFragment);
            };
            let key_frame = frame.flags & MEDIA_DATAGRAM_FLAG_KEY_FRAME != 0;
            match state.accept(&fragment, key_frame) {
                Ok(complete) => {
                    if complete {
                        publication
                            .fragment_bytes_seen
                            .retain(|(sequence, _)| *sequence != frame.sequence);
                    }
                }
                Err(VideoDropReason::DuplicateFrame) => {
                    return VideoAdmission::Drop(VideoDropReason::DuplicateFrame);
                }
                Err(reason) => {
                    publication.has_key_frame = false;
                    publication.reject_sequence(frame.sequence, reason);
                    return VideoAdmission::Drop(reason);
                }
            }
            return VideoAdmission::Forward;
        }
        if let Some(decision) = publication.decision_for(frame.sequence) {
            if decision == VideoAdmission::Forward
                && let Some((_, state)) = publication
                    .fragment_bytes_seen
                    .iter_mut()
                    .find(|(sequence, _)| *sequence == frame.sequence)
            {
                let key_frame = frame.flags & MEDIA_DATAGRAM_FLAG_KEY_FRAME != 0;
                if !fragment.is_fragmented
                    || state.accept(&fragment, key_frame) == Err(VideoDropReason::MalformedFragment)
                {
                    publication.has_key_frame = false;
                    publication.reject_sequence(frame.sequence, VideoDropReason::MalformedFragment);
                    return VideoAdmission::Drop(VideoDropReason::MalformedFragment);
                }
            }
            return match decision {
                VideoAdmission::Forward => VideoAdmission::Drop(VideoDropReason::DuplicateFrame),
                rejected => rejected,
            };
        }

        let is_key_frame = frame.flags & MEDIA_DATAGRAM_FLAG_KEY_FRAME != 0;
        let decision = publication.inspect_frame(
            frame.sequence,
            is_key_frame,
            fragment.vp9_payload,
            fragment.frame_bytes,
            bitrate_limit,
            now,
        );
        publication.remember(frame.sequence, decision);
        if fragment.is_fragmented && decision == VideoAdmission::Forward {
            if fragment.count > 1 {
                publication
                    .fragment_bytes_seen
                    .push((frame.sequence, FragmentState::new(&fragment, is_key_frame)));
            } else if fragment.vp9_payload.len() != fragment.frame_bytes {
                publication.has_key_frame = false;
                publication.reject_sequence(frame.sequence, VideoDropReason::MalformedFragment);
                return VideoAdmission::Drop(VideoDropReason::MalformedFragment);
            }
        }
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

    /// Сбрасывает состояние остановленного источника, сохраняя его бюджет до выхода из комнаты.
    pub(super) fn remove_source(
        &mut self,
        session_id: Uuid,
        room_id: Uuid,
        kind: MediaDatagramKind,
    ) {
        for publication in self.publications.iter_mut().filter(|publication| {
            publication.key.session_id == session_id
                && publication.key.room_id == room_id
                && publication.key.kind == kind
        }) {
            publication.has_key_frame = false;
            publication.recent_decisions.clear();
            publication.fragment_bytes_seen.clear();
            publication.highest_sequence = None;
            publication.last_forward_at = None;
        }
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

/// Заимствованные поля видеокадра для проверки policy без создания датаграммы.
struct VideoFrameView<'a> {
    session_id: Uuid,
    kind: MediaDatagramKind,
    flags: u8,
    sequence: u64,
    room_id: Uuid,
    payload: &'a [u8],
}

impl InMemoryVoicePresenceStore {
    pub(super) async fn inspect_video_datagram(
        &self,
        session_id: Uuid,
        datagram: &MediaDatagram,
        bitrate_limit: VideoBitrateLimit,
    ) -> VideoAdmission {
        self.video_publications
            .lock()
            .await
            .inspect(session_id, datagram, bitrate_limit)
    }

    /// Применяет video policy к payload из исходного WebTransport-буфера.
    pub(super) async fn inspect_video_payload(
        &self,
        session_id: Uuid,
        header: &MediaDatagramHeader,
        payload: &[u8],
        bitrate_limit: VideoBitrateLimit,
    ) -> VideoAdmission {
        self.video_publications.lock().await.inspect_header_at(
            session_id,
            header,
            payload,
            bitrate_limit,
            Instant::now(),
        )
    }

    /// Сбрасывает остановленный видеоисточник, сохраняя учёт его бюджета.
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
    has_key_frame: bool,
    budget_bits: Option<i128>,
    last_budget_update: Instant,
    recent_decisions: Vec<(u64, VideoAdmission)>,
    highest_sequence: Option<u64>,
    fragment_bytes_seen: Vec<(u64, FragmentState)>,
    last_forward_at: Option<Instant>,
}

impl VideoPublication {
    fn new(key: VideoPublicationKey, now: Instant) -> Self {
        Self {
            key,
            has_key_frame: false,
            budget_bits: None,
            last_budget_update: now,
            recent_decisions: Vec::new(),
            highest_sequence: None,
            fragment_bytes_seen: Vec::new(),
            last_forward_at: None,
        }
    }

    fn inspect_frame(
        &mut self,
        _sequence: u64,
        is_key_frame: bool,
        vp9_payload: &[u8],
        frame_bytes: usize,
        bitrate_limit: VideoBitrateLimit,
        now: Instant,
    ) -> VideoAdmission {
        if Some(bitrate_limit.source) != source_for_kind(self.key.kind) {
            self.has_key_frame = false;
            return VideoAdmission::Drop(VideoDropReason::SourceNotAllowed);
        }
        let max_bitrate_bps = bitrate_limit.max_bitrate_bps;
        if max_bitrate_bps == 0 {
            self.has_key_frame = false;
            return VideoAdmission::Drop(VideoDropReason::SourceNotAllowed);
        };

        if is_key_frame {
            let Some((width, height)) = parse_key_frame_dimensions(vp9_payload) else {
                self.has_key_frame = false;
                return VideoAdmission::Drop(VideoDropReason::InvalidVp9KeyFrame);
            };
            if !self.has_key_frame {
                tracing::debug!(
                    session_id = %self.key.session_id,
                    room_id = %self.key.room_id,
                    kind = ?self.key.kind,
                    width,
                    height,
                    max_bitrate_bps,
                    "accepted video publication keyframe dimensions without resolution limit"
                );
            }
        } else if !self.has_key_frame {
            return VideoAdmission::Drop(VideoDropReason::AwaitingKeyFrame);
        }

        let capacity_bits = i128::from(max_bitrate_bps) * i128::from(VIDEO_BURST_SECONDS);
        let elapsed_ns = now
            .saturating_duration_since(self.last_budget_update)
            .as_nanos();
        let replenished_bits =
            elapsed_ns.saturating_mul(u128::from(max_bitrate_bps)) / 1_000_000_000;
        let budget = self.budget_bits.unwrap_or(capacity_bits);
        let available_bits = budget
            .saturating_add(i128::try_from(replenished_bits).unwrap_or(i128::MAX))
            .min(capacity_bits);
        self.last_budget_update = now;
        self.budget_bits = Some(available_bits);

        let required_bits = i128::try_from(frame_bytes)
            .unwrap_or(i128::MAX)
            .saturating_mul(8);
        // Первый keyframe нового источника и keyframe восстановления могут занять
        // ещё до двух секунд бюджета; новый источник начинает с полного bucket.
        // Долг погашается будущим трафиком и не позволяет повторять burst бесплатно.
        let minimum_budget = if is_key_frame && !self.has_key_frame {
            -capacity_bits
        } else {
            0
        };
        let remaining = available_bits.saturating_sub(required_bits);
        if remaining < minimum_budget {
            self.has_key_frame = false;
            return VideoAdmission::Drop(VideoDropReason::BitrateLimitExceeded {
                max_bitrate_bps,
                frame_bytes,
            });
        }
        if remaining < 0 {
            tracing::debug!(session_id = %self.key.session_id, room_id = %self.key.room_id, kind = ?self.key.kind, debt_bits = -remaining, "video recovery keyframe borrowed bitrate budget");
        }
        self.budget_bits = Some(remaining);
        self.has_key_frame = true;
        VideoAdmission::Forward
    }

    fn decision_for(&self, sequence: u64) -> Option<VideoAdmission> {
        self.recent_decisions
            .iter()
            .rev()
            .find_map(|(candidate, decision)| (*candidate == sequence).then_some(*decision))
    }

    fn reject_sequence(&mut self, sequence: u64, reason: VideoDropReason) {
        if let Some((_, decision)) = self
            .recent_decisions
            .iter_mut()
            .find(|(candidate, _)| *candidate == sequence)
        {
            *decision = VideoAdmission::Drop(reason);
        }
        self.fragment_bytes_seen
            .retain(|(candidate, _)| *candidate != sequence);
    }

    fn remember(&mut self, sequence: u64, decision: VideoAdmission) {
        self.highest_sequence = Some(self.highest_sequence.map_or(sequence, |v| v.max(sequence)));
        if self.recent_decisions.len() == RECENT_SEQUENCE_LIMIT {
            let (expired_sequence, _) = self.recent_decisions.remove(0);
            self.fragment_bytes_seen
                .retain(|(candidate, _)| *candidate != expired_sequence);
        }
        self.recent_decisions.push((sequence, decision));
    }
}

fn source_for_kind(kind: MediaDatagramKind) -> Option<VideoStreamSource> {
    match kind {
        MediaDatagramKind::CameraFrame => Some(VideoStreamSource::Camera),
        MediaDatagramKind::ScreenFrame => Some(VideoStreamSource::ScreenShare),
        MediaDatagramKind::VoiceFrame => None,
    }
}

#[cfg(test)]
pub(crate) mod tests;

//! Фрагментация и сборка видеомедиадатаграмм голосовой комнаты.

use std::collections::HashMap;

use cheenhub_contracts::media::{
    MEDIA_DATAGRAM_FLAG_FRAGMENTED, MEDIA_DATAGRAM_FLAG_KEY_FRAME, MediaCodec, MediaDatagram,
    MediaDatagramKind,
};
use dioxus::prelude::{debug, warn};
use uuid::Uuid;

use crate::features::realtime::RealtimeError;

const VIDEO_FRAME_FRAGMENT_BYTES: usize = 900;
const VIDEO_FRAME_FRAGMENT_HEADER_LEN: usize = 8;
const VIDEO_FRAME_FRAGMENT_LOG_INTERVAL: u64 = 300;
const MAX_VIDEO_FRAME_BYTES: usize = 2 * 1024 * 1024;
const MAX_PENDING_VIDEO_FRAMES: usize = 32;

/// Закодированный видеокадр, готовый к отправке медиадатаграммами.
pub(super) struct OutboundVideoFrame {
    /// Локальный для отправителя номер кадра.
    pub(super) sequence: u64,
    /// Временная метка кадра в микросекундах.
    pub(super) timestamp_us: u64,
    /// Длительность кадра в микросекундах.
    pub(super) duration_us: u32,
    /// Может ли этот кадр открыть поток декодера.
    pub(super) key_frame: bool,
    /// Сырые байты закодированного кадра.
    pub(super) bytes: Vec<u8>,
}

/// Возвращает, что датаграмма содержит один фрагмент более крупного видеокадра.
pub(super) fn is_fragmented(datagram: &MediaDatagram) -> bool {
    datagram.flags & MEDIA_DATAGRAM_FLAG_FRAGMENTED != 0
}

/// Делит один закодированный видеокадр на payload'ы безопасного размера для датаграмм.
pub(super) fn video_frame_datagrams(
    room_id: Uuid,
    kind: MediaDatagramKind,
    codec: MediaCodec,
    frame: OutboundVideoFrame,
) -> Result<Vec<MediaDatagram>, RealtimeError> {
    let flags = if frame.key_frame {
        MEDIA_DATAGRAM_FLAG_KEY_FRAME
    } else {
        0
    };
    if frame.bytes.len() <= VIDEO_FRAME_FRAGMENT_BYTES {
        return Ok(vec![MediaDatagram {
            kind,
            codec,
            flags,
            sequence: frame.sequence,
            timestamp_us: frame.timestamp_us,
            duration_us: frame.duration_us,
            room_id,
            sender_user_id: Uuid::nil(),
            payload: frame.bytes,
        }]);
    }

    let payload_len = frame.bytes.len();
    let fragment_count = payload_len.div_ceil(VIDEO_FRAME_FRAGMENT_BYTES);
    let fragment_count = u16::try_from(fragment_count).map_err(|_| {
        RealtimeError::new(format!(
            "Video frame requires too many datagram fragments: {fragment_count}"
        ))
    })?;
    let payload_len = u32::try_from(payload_len).map_err(|_| {
        RealtimeError::new(format!(
            "Video frame is too large for datagram fragmentation: {payload_len} bytes"
        ))
    })?;
    if frame.key_frame
        && frame
            .sequence
            .is_multiple_of(VIDEO_FRAME_FRAGMENT_LOG_INTERVAL)
    {
        debug!(
            sequence = frame.sequence,
            payload_bytes = payload_len,
            fragment_count,
            fragment_payload_bytes = VIDEO_FRAME_FRAGMENT_BYTES,
            kind = ?kind,
            "sending fragmented video key frame"
        );
    }

    let datagrams = frame
        .bytes
        .chunks(VIDEO_FRAME_FRAGMENT_BYTES)
        .enumerate()
        .map(|(fragment_index, chunk)| {
            let fragment_index = u16::try_from(fragment_index)
                .map_err(|_| RealtimeError::new("Video frame fragment index overflowed u16."))?;
            let mut payload = Vec::with_capacity(VIDEO_FRAME_FRAGMENT_HEADER_LEN + chunk.len());
            payload.extend_from_slice(&payload_len.to_be_bytes());
            payload.extend_from_slice(&fragment_index.to_be_bytes());
            payload.extend_from_slice(&fragment_count.to_be_bytes());
            payload.extend_from_slice(chunk);

            Ok(MediaDatagram {
                kind,
                codec,
                flags: flags | MEDIA_DATAGRAM_FLAG_FRAGMENTED,
                sequence: frame.sequence,
                timestamp_us: frame.timestamp_us,
                duration_us: frame.duration_us,
                room_id,
                sender_user_id: Uuid::nil(),
                payload,
            })
        })
        .collect::<Result<Vec<_>, RealtimeError>>()?;

    Ok(datagrams)
}

/// Собирает фрагментированные видеодатаграммы в полные медиадатаграммы.
#[derive(Default)]
pub(super) struct VideoFrameReassembler {
    pending: HashMap<VideoFrameKey, PendingVideoFrame>,
}

impl VideoFrameReassembler {
    /// Добавляет фрагмент и возвращает полный кадр, когда пришли все фрагменты.
    pub(super) fn push(&mut self, datagram: MediaDatagram) -> Option<MediaDatagram> {
        let Some(fragment) = VideoFrameFragment::decode(&datagram.payload) else {
            warn!(
                sender_user_id = %datagram.sender_user_id,
                sequence = datagram.sequence,
                kind = ?datagram.kind,
                "dropping malformed video frame fragment"
            );
            return None;
        };
        if fragment.total_len > MAX_VIDEO_FRAME_BYTES {
            warn!(
                sender_user_id = %datagram.sender_user_id,
                sequence = datagram.sequence,
                total_bytes = fragment.total_len,
                kind = ?datagram.kind,
                "dropping oversized fragmented video frame"
            );
            return None;
        }

        self.drop_stale_frames(&datagram);
        let key = VideoFrameKey {
            room_id: datagram.room_id,
            sender_user_id: datagram.sender_user_id,
            kind: datagram.kind,
            sequence: datagram.sequence,
        };
        let pending = self.pending.entry(key.clone()).or_insert_with(|| {
            let fragments = std::iter::repeat_with(|| None)
                .take(fragment.count)
                .collect();
            PendingVideoFrame {
                datagram: MediaDatagram {
                    payload: Vec::new(),
                    ..datagram.clone()
                },
                total_len: fragment.total_len,
                received: 0,
                fragments,
            }
        });
        if pending.total_len != fragment.total_len || pending.fragments.len() != fragment.count {
            warn!(
                sender_user_id = %datagram.sender_user_id,
                sequence = datagram.sequence,
                kind = ?datagram.kind,
                "dropping inconsistent video frame fragments"
            );
            self.pending.remove(&key);
            return None;
        }
        if pending.fragments[fragment.index].is_none() {
            pending.received = pending.received.saturating_add(1);
            pending.fragments[fragment.index] = Some(fragment.bytes);
        }
        if pending.received != pending.fragments.len() {
            return None;
        }

        let mut pending = self.pending.remove(&key)?;
        let mut bytes = Vec::with_capacity(pending.total_len);
        for fragment in pending.fragments.drain(..) {
            bytes.extend(fragment?);
        }
        if bytes.len() != pending.total_len {
            warn!(
                sender_user_id = %pending.datagram.sender_user_id,
                sequence = pending.datagram.sequence,
                expected_bytes = pending.total_len,
                actual_bytes = bytes.len(),
                kind = ?pending.datagram.kind,
                "dropping reassembled video frame with invalid size"
            );
            return None;
        }
        pending.datagram.flags &= !MEDIA_DATAGRAM_FLAG_FRAGMENTED;
        pending.datagram.payload = bytes;
        if pending.datagram.flags & MEDIA_DATAGRAM_FLAG_KEY_FRAME != 0
            && pending
                .datagram
                .sequence
                .is_multiple_of(VIDEO_FRAME_FRAGMENT_LOG_INTERVAL)
        {
            debug!(
                sender_user_id = %pending.datagram.sender_user_id,
                sequence = pending.datagram.sequence,
                payload_bytes = pending.datagram.payload.len(),
                fragments = pending.received,
                kind = ?pending.datagram.kind,
                "reassembled video key frame"
            );
        }

        Some(pending.datagram)
    }

    fn drop_stale_frames(&mut self, datagram: &MediaDatagram) {
        if self.pending.len() < MAX_PENDING_VIDEO_FRAMES {
            return;
        }

        let sender_user_id = datagram.sender_user_id;
        let kind = datagram.kind;
        let sequence = datagram.sequence;
        self.pending.retain(|key, _| {
            key.sender_user_id != sender_user_id
                || key.kind != kind
                || key.sequence.saturating_add(8) >= sequence
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VideoFrameKey {
    room_id: Uuid,
    sender_user_id: Uuid,
    kind: MediaDatagramKind,
    sequence: u64,
}

struct PendingVideoFrame {
    datagram: MediaDatagram,
    total_len: usize,
    received: usize,
    fragments: Vec<Option<Vec<u8>>>,
}

struct VideoFrameFragment {
    total_len: usize,
    index: usize,
    count: usize,
    bytes: Vec<u8>,
}

impl VideoFrameFragment {
    fn decode(payload: &[u8]) -> Option<Self> {
        if payload.len() < VIDEO_FRAME_FRAGMENT_HEADER_LEN {
            return None;
        }

        let total_len = u32::from_be_bytes(payload[0..4].try_into().ok()?) as usize;
        let index = u16::from_be_bytes(payload[4..6].try_into().ok()?) as usize;
        let count = u16::from_be_bytes(payload[6..8].try_into().ok()?) as usize;
        if count == 0 || index >= count {
            return None;
        }

        Some(Self {
            total_len,
            index,
            count,
            bytes: payload[VIDEO_FRAME_FRAGMENT_HEADER_LEN..].to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reassembles_fragmented_camera_key_frame() {
        let room_id = Uuid::new_v4();
        let sender_user_id = Uuid::new_v4();
        let mut reassembler = VideoFrameReassembler::default();
        let second = fragmented_video_datagram(FragmentFixture {
            room_id,
            sender_user_id,
            kind: MediaDatagramKind::CameraFrame,
            sequence: 7,
            flags: MEDIA_DATAGRAM_FLAG_KEY_FRAME,
            total_len: 5,
            fragment_index: 1,
            fragment_count: 2,
            bytes: &[4, 5],
        });
        let first = fragmented_video_datagram(FragmentFixture {
            room_id,
            sender_user_id,
            kind: MediaDatagramKind::CameraFrame,
            sequence: 7,
            flags: MEDIA_DATAGRAM_FLAG_KEY_FRAME,
            total_len: 5,
            fragment_index: 0,
            fragment_count: 2,
            bytes: &[1, 2, 3],
        });

        assert!(reassembler.push(second).is_none());
        let datagram = reassembler.push(first).expect("frame reassembles");

        assert_eq!(datagram.room_id, room_id);
        assert_eq!(datagram.sender_user_id, sender_user_id);
        assert_eq!(datagram.kind, MediaDatagramKind::CameraFrame);
        assert_eq!(datagram.sequence, 7);
        assert_eq!(datagram.flags, MEDIA_DATAGRAM_FLAG_KEY_FRAME);
        assert_eq!(datagram.payload, vec![1, 2, 3, 4, 5]);
    }

    struct FragmentFixture<'a> {
        room_id: Uuid,
        sender_user_id: Uuid,
        kind: MediaDatagramKind,
        sequence: u64,
        flags: u8,
        total_len: u32,
        fragment_index: u16,
        fragment_count: u16,
        bytes: &'a [u8],
    }

    fn fragmented_video_datagram(fragment: FragmentFixture<'_>) -> MediaDatagram {
        let mut payload = Vec::new();
        payload.extend_from_slice(&fragment.total_len.to_be_bytes());
        payload.extend_from_slice(&fragment.fragment_index.to_be_bytes());
        payload.extend_from_slice(&fragment.fragment_count.to_be_bytes());
        payload.extend_from_slice(fragment.bytes);

        MediaDatagram {
            kind: fragment.kind,
            codec: MediaCodec::Vp9,
            flags: fragment.flags | MEDIA_DATAGRAM_FLAG_FRAGMENTED,
            sequence: fragment.sequence,
            timestamp_us: 100,
            duration_us: 33_333,
            room_id: fragment.room_id,
            sender_user_id: fragment.sender_user_id,
            payload,
        }
    }
}

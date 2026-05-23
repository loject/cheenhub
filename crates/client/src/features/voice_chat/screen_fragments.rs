//! Screen sharing media datagram fragmentation and reassembly.

use std::collections::HashMap;

use cheenhub_contracts::media::{
    MEDIA_DATAGRAM_FLAG_FRAGMENTED, MEDIA_DATAGRAM_FLAG_KEY_FRAME, MediaCodec, MediaDatagram,
    MediaDatagramKind,
};
use dioxus::prelude::{debug, warn};
use uuid::Uuid;

use crate::features::realtime::RealtimeError;
use crate::features::screen_share::EncodedScreenShareFrame;

const SCREEN_FRAME_FRAGMENT_BYTES: usize = 900;
const SCREEN_FRAME_FRAGMENT_HEADER_LEN: usize = 8;
const SCREEN_FRAME_FRAGMENT_LOG_INTERVAL: u64 = 300;
const MAX_SCREEN_FRAME_BYTES: usize = 2 * 1024 * 1024;
const MAX_PENDING_SCREEN_FRAMES: usize = 32;

/// Returns whether this datagram carries one fragment of a larger screen frame.
pub(super) fn is_fragmented(datagram: &MediaDatagram) -> bool {
    datagram.flags & MEDIA_DATAGRAM_FLAG_FRAGMENTED != 0
}

/// Splits one encoded screen sharing frame into datagram-sized payloads.
pub(super) fn screen_frame_datagrams(
    room_id: Uuid,
    codec: MediaCodec,
    flags: u8,
    frame: EncodedScreenShareFrame,
) -> Result<Vec<MediaDatagram>, RealtimeError> {
    if frame.bytes.len() <= SCREEN_FRAME_FRAGMENT_BYTES {
        return Ok(vec![MediaDatagram {
            kind: MediaDatagramKind::ScreenFrame,
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
    let fragment_count = payload_len.div_ceil(SCREEN_FRAME_FRAGMENT_BYTES);
    let fragment_count = u16::try_from(fragment_count).map_err(|_| {
        RealtimeError::new(format!(
            "Screen frame requires too many datagram fragments: {fragment_count}"
        ))
    })?;
    let payload_len = u32::try_from(payload_len).map_err(|_| {
        RealtimeError::new(format!(
            "Screen frame is too large for datagram fragmentation: {payload_len} bytes"
        ))
    })?;
    if frame.key_frame
        && frame
            .sequence
            .is_multiple_of(SCREEN_FRAME_FRAGMENT_LOG_INTERVAL)
    {
        debug!(
            sequence = frame.sequence,
            payload_bytes = payload_len,
            fragment_count,
            fragment_payload_bytes = SCREEN_FRAME_FRAGMENT_BYTES,
            "sending fragmented screen sharing key frame"
        );
    }

    let datagrams = frame
        .bytes
        .chunks(SCREEN_FRAME_FRAGMENT_BYTES)
        .enumerate()
        .map(|(fragment_index, chunk)| {
            let fragment_index = u16::try_from(fragment_index)
                .map_err(|_| RealtimeError::new("Screen frame fragment index overflowed u16."))?;
            let mut payload = Vec::with_capacity(SCREEN_FRAME_FRAGMENT_HEADER_LEN + chunk.len());
            payload.extend_from_slice(&payload_len.to_be_bytes());
            payload.extend_from_slice(&fragment_index.to_be_bytes());
            payload.extend_from_slice(&fragment_count.to_be_bytes());
            payload.extend_from_slice(chunk);

            Ok(MediaDatagram {
                kind: MediaDatagramKind::ScreenFrame,
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

/// Reassembles fragmented screen sharing datagrams into complete media datagrams.
#[derive(Default)]
pub(super) struct ScreenFrameReassembler {
    pending: HashMap<ScreenFrameKey, PendingScreenFrame>,
}

impl ScreenFrameReassembler {
    /// Pushes one fragment and returns a complete frame when all fragments have arrived.
    pub(super) fn push(&mut self, datagram: MediaDatagram) -> Option<MediaDatagram> {
        let Some(fragment) = ScreenFrameFragment::decode(&datagram.payload) else {
            warn!(
                sender_user_id = %datagram.sender_user_id,
                sequence = datagram.sequence,
                "dropping malformed screen sharing frame fragment"
            );
            return None;
        };
        if fragment.total_len > MAX_SCREEN_FRAME_BYTES {
            warn!(
                sender_user_id = %datagram.sender_user_id,
                sequence = datagram.sequence,
                total_bytes = fragment.total_len,
                "dropping oversized fragmented screen sharing frame"
            );
            return None;
        }

        self.drop_stale_frames(&datagram);
        let key = ScreenFrameKey {
            room_id: datagram.room_id,
            sender_user_id: datagram.sender_user_id,
            sequence: datagram.sequence,
        };
        let pending = self.pending.entry(key.clone()).or_insert_with(|| {
            let fragments = std::iter::repeat_with(|| None)
                .take(fragment.count)
                .collect();
            PendingScreenFrame {
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
                "dropping inconsistent screen sharing frame fragments"
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
                "dropping reassembled screen sharing frame with invalid size"
            );
            return None;
        }
        pending.datagram.flags &= !MEDIA_DATAGRAM_FLAG_FRAGMENTED;
        pending.datagram.payload = bytes;
        if pending.datagram.flags & MEDIA_DATAGRAM_FLAG_KEY_FRAME != 0
            && pending
                .datagram
                .sequence
                .is_multiple_of(SCREEN_FRAME_FRAGMENT_LOG_INTERVAL)
        {
            debug!(
                sender_user_id = %pending.datagram.sender_user_id,
                sequence = pending.datagram.sequence,
                payload_bytes = pending.datagram.payload.len(),
                fragments = pending.received,
                "reassembled screen sharing key frame"
            );
        }

        Some(pending.datagram)
    }

    fn drop_stale_frames(&mut self, datagram: &MediaDatagram) {
        if self.pending.len() < MAX_PENDING_SCREEN_FRAMES {
            return;
        }

        let sender_user_id = datagram.sender_user_id;
        let sequence = datagram.sequence;
        self.pending.retain(|key, _| {
            key.sender_user_id != sender_user_id || key.sequence.saturating_add(8) >= sequence
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ScreenFrameKey {
    room_id: Uuid,
    sender_user_id: Uuid,
    sequence: u64,
}

struct PendingScreenFrame {
    datagram: MediaDatagram,
    total_len: usize,
    received: usize,
    fragments: Vec<Option<Vec<u8>>>,
}

struct ScreenFrameFragment {
    total_len: usize,
    index: usize,
    count: usize,
    bytes: Vec<u8>,
}

impl ScreenFrameFragment {
    fn decode(payload: &[u8]) -> Option<Self> {
        if payload.len() < SCREEN_FRAME_FRAGMENT_HEADER_LEN {
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
            bytes: payload[SCREEN_FRAME_FRAGMENT_HEADER_LEN..].to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reassembles_fragmented_screen_key_frame() {
        let room_id = Uuid::new_v4();
        let sender_user_id = Uuid::new_v4();
        let mut reassembler = ScreenFrameReassembler::default();
        let second = fragmented_screen_datagram(FragmentFixture {
            room_id,
            sender_user_id,
            sequence: 7,
            flags: MEDIA_DATAGRAM_FLAG_KEY_FRAME,
            total_len: 5,
            fragment_index: 1,
            fragment_count: 2,
            bytes: &[4, 5],
        });
        let first = fragmented_screen_datagram(FragmentFixture {
            room_id,
            sender_user_id,
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
        assert_eq!(datagram.sequence, 7);
        assert_eq!(datagram.flags, MEDIA_DATAGRAM_FLAG_KEY_FRAME);
        assert_eq!(datagram.payload, vec![1, 2, 3, 4, 5]);
    }

    struct FragmentFixture<'a> {
        room_id: Uuid,
        sender_user_id: Uuid,
        sequence: u64,
        flags: u8,
        total_len: u32,
        fragment_index: u16,
        fragment_count: u16,
        bytes: &'a [u8],
    }

    fn fragmented_screen_datagram(fragment: FragmentFixture<'_>) -> MediaDatagram {
        let mut payload = Vec::new();
        payload.extend_from_slice(&fragment.total_len.to_be_bytes());
        payload.extend_from_slice(&fragment.fragment_index.to_be_bytes());
        payload.extend_from_slice(&fragment.fragment_count.to_be_bytes());
        payload.extend_from_slice(fragment.bytes);

        MediaDatagram {
            kind: MediaDatagramKind::ScreenFrame,
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

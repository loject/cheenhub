//! Проверка заголовков и индексов фрагментов одного видеокадра.

use super::VideoDropReason;
use cheenhub_contracts::media::MEDIA_DATAGRAM_FLAG_FRAGMENTED;

const HEADER_LEN: usize = 8;

pub(super) struct FrameFragment<'a> {
    pub(super) index: u16,
    pub(super) count: u16,
    pub(super) is_fragmented: bool,
    pub(super) vp9_payload: &'a [u8],
    pub(super) frame_bytes: usize,
}

pub(super) fn frame_fragment(
    flags: u8,
    payload: &[u8],
) -> Result<FrameFragment<'_>, VideoDropReason> {
    if flags & MEDIA_DATAGRAM_FLAG_FRAGMENTED == 0 {
        return Ok(FrameFragment {
            index: 0,
            count: 1,
            is_fragmented: false,
            vp9_payload: payload,
            frame_bytes: payload.len(),
        });
    }
    if payload.len() < HEADER_LEN {
        return Err(VideoDropReason::MalformedFragment);
    }
    let frame_bytes = u32::from_be_bytes(payload[..4].try_into().unwrap()) as usize;
    let index = u16::from_be_bytes(payload[4..6].try_into().unwrap());
    let count = u16::from_be_bytes(payload[6..8].try_into().unwrap());
    if count == 0
        || index >= count
        || frame_bytes < usize::from(count)
        || payload.len() == HEADER_LEN
        || payload.len() - HEADER_LEN > frame_bytes
    {
        return Err(VideoDropReason::MalformedFragment);
    }
    Ok(FrameFragment {
        index,
        count,
        is_fragmented: true,
        vp9_payload: &payload[HEADER_LEN..],
        frame_bytes,
    })
}

/// Bitmap ограничен значением u16 в wire-заголовке: не более 8192 байт на кадр.
pub(super) struct FragmentState {
    count: u16,
    frame_bytes: usize,
    seen_bytes: usize,
    seen_count: u16,
    key_frame: bool,
    seen: Vec<u64>,
}

impl FragmentState {
    pub(super) fn new(first: &FrameFragment<'_>, key_frame: bool) -> Self {
        let mut seen = vec![0; usize::from(first.count).div_ceil(64)];
        seen[0] = 1;
        Self {
            count: first.count,
            frame_bytes: first.frame_bytes,
            seen_bytes: first.vp9_payload.len(),
            seen_count: 1,
            key_frame,
            seen,
        }
    }

    pub(super) fn accept(
        &mut self,
        fragment: &FrameFragment<'_>,
        key_frame: bool,
    ) -> Result<bool, VideoDropReason> {
        if self.count != fragment.count
            || self.frame_bytes != fragment.frame_bytes
            || self.key_frame != key_frame
        {
            return Err(VideoDropReason::MalformedFragment);
        }
        let index = usize::from(fragment.index);
        let mask = 1_u64 << (index % 64);
        if self.seen[index / 64] & mask != 0 {
            return Err(VideoDropReason::DuplicateFrame);
        }
        let total = self
            .seen_bytes
            .checked_add(fragment.vp9_payload.len())
            .ok_or(VideoDropReason::MalformedFragment)?;
        if total > self.frame_bytes
            || (self.seen_count + 1 == self.count && total != self.frame_bytes)
        {
            return Err(VideoDropReason::MalformedFragment);
        }
        self.seen[index / 64] |= mask;
        self.seen_count += 1;
        self.seen_bytes = total;
        Ok(self.seen_count == self.count)
    }
}

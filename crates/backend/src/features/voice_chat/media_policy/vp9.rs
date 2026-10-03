//! Чтение размеров кадра из VP9 uncompressed header.

/// Возвращает закодированные размеры VP9 key frame.
pub(super) fn parse_key_frame_dimensions(payload: &[u8]) -> Option<(u32, u32)> {
    let mut bits = BitReader::new(payload);
    (bits.read(2)? == 0b10).then_some(())?;
    let profile = bits.read(1)? | (bits.read(1)? << 1);
    if profile == 3 {
        (bits.read(1)? == 0).then_some(())?;
    }
    (bits.read(1)? == 0).then_some(())?; // show_existing_frame
    (bits.read(1)? == 0).then_some(())?; // frame_type: key frame
    bits.read(1)?; // show_frame
    bits.read(1)?; // error_resilient_mode
    (bits.read(24)? == 0x49_83_42).then_some(())?;

    if profile >= 2 {
        bits.read(1)?; // ten_or_twelve_bit
    }
    let color_space = bits.read(3)?;
    if color_space != 7 {
        bits.read(1)?; // color_range
        if profile == 1 || profile == 3 {
            bits.read(1)?; // subsampling_x
            bits.read(1)?; // subsampling_y
            (bits.read(1)? == 0).then_some(())?;
        }
    } else if profile == 1 || profile == 3 {
        // Для sRGB subsampling фиксирован в 4:4:4, в header остаётся reserved bit.
        (bits.read(1)? == 0).then_some(())?;
    }

    let width = bits.read(16)?.checked_add(1)?;
    let height = bits.read(16)?.checked_add(1)?;
    Some((width, height))
}

struct BitReader<'a> {
    bytes: &'a [u8],
    bit_offset: usize,
}

impl<'a> BitReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            bit_offset: 0,
        }
    }

    fn read(&mut self, count: usize) -> Option<u32> {
        let mut value = 0_u32;
        for _ in 0..count {
            let byte = *self.bytes.get(self.bit_offset / 8)?;
            let shift = 7 - self.bit_offset % 8;
            value = (value << 1) | u32::from((byte >> shift) & 1);
            self.bit_offset += 1;
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests;

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
mod tests {
    use super::parse_key_frame_dimensions;

    #[test]
    fn parses_profile_zero_vp9_key_frame_dimensions() {
        let payload = vp9_key_frame(1280, 720);
        assert_eq!(parse_key_frame_dimensions(&payload), Some((1280, 720)));
    }

    #[test]
    fn parses_known_profile_zero_header_bytes() {
        // Header получен тем же порядком полей, который выдаёт VP9 profile 0:
        // frame marker/profile/key flag, sync code, BT.709 limited range и 1920x1080.
        let payload = [0x82, 0x49, 0x83, 0x42, 0x20, 0x77, 0xf0, 0x43, 0x70];
        assert_eq!(parse_key_frame_dimensions(&payload), Some((1920, 1080)));
    }

    #[test]
    fn rejects_inter_frame_as_key_frame_header() {
        let mut payload = vp9_key_frame(1280, 720);
        payload[0] |= 0b0000_0100;
        assert_eq!(parse_key_frame_dimensions(&payload), None);
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

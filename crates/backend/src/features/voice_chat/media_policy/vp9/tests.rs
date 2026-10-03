//! Проверки разбора VP9-кадров.

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

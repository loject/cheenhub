use super::CpalOutputSample;

#[test]
fn converts_f32_without_overflow() {
    assert_eq!(f32::from_f32(2.0), 1.0);
    assert_eq!(f32::from_f32(-2.0), -1.0);
}

#[test]
fn converts_unsigned_midpoint_to_silence() {
    assert_eq!(u8::from_f32(0.0), 128);
    assert_eq!(u16::from_f32(0.0), 32_768);
}

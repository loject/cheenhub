use super::*;
use cpal::{SupportedBufferSize, SupportedStreamConfigRange};

fn config_range(
    sample_format: SampleFormat,
    channels: u16,
    rate: u32,
) -> SupportedStreamConfigRange {
    SupportedStreamConfigRange::new(
        channels,
        rate,
        rate,
        SupportedBufferSize::Unknown,
        sample_format,
    )
}

#[test]
fn playback_prefers_float_over_first_eight_bit_format() {
    let configs = [
        config_range(SampleFormat::U8, 2, 48_000),
        config_range(SampleFormat::I16, 2, 48_000),
        config_range(SampleFormat::F32, 2, 48_000),
    ];

    let selected = select_preferred_output_config(configs, 48_000).unwrap();

    assert_eq!(selected.sample_format(), SampleFormat::F32);
    assert_eq!(selected.channels(), 2);
}

#[test]
fn playback_prefers_stereo_when_high_quality_formats_differ() {
    let configs = [
        config_range(SampleFormat::F32, 6, 48_000),
        config_range(SampleFormat::I16, 2, 48_000),
    ];

    let selected = select_preferred_output_config(configs, 48_000).unwrap();

    assert_eq!(selected.sample_format(), SampleFormat::I16);
    assert_eq!(selected.channels(), 2);
}

#[test]
fn playback_retains_eight_bit_when_it_is_the_only_format() {
    let selected =
        select_preferred_output_config([config_range(SampleFormat::U8, 2, 48_000)], 48_000)
            .unwrap();

    assert_eq!(selected.sample_format(), SampleFormat::U8);
}

#[test]
fn playback_ignores_unsupported_format_and_rate() {
    let configs = [
        config_range(SampleFormat::I64, 2, 48_000),
        config_range(SampleFormat::F32, 2, 44_100),
        config_range(SampleFormat::I16, 2, 48_000),
    ];

    let selected = select_preferred_output_config(configs, 48_000).unwrap();

    assert_eq!(selected.sample_format(), SampleFormat::I16);
}

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

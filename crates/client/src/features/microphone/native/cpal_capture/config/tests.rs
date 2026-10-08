use super::*;
use cpal::{SampleFormat, SupportedBufferSize};

fn range(format: SampleFormat, channels: u16, rate: u32) -> SupportedStreamConfigRange {
    SupportedStreamConfigRange::new(
        channels,
        SampleRate(rate),
        SampleRate(rate),
        SupportedBufferSize::Unknown,
        format,
    )
}

#[test]
fn capture_prefers_float_over_first_eight_bit_format() {
    let configs = [
        range(SampleFormat::U8, 1, 48_000),
        range(SampleFormat::I16, 1, 48_000),
        range(SampleFormat::F32, 1, 48_000),
    ];

    let selected = select(configs, SampleRate(48_000), 1).unwrap();

    assert_eq!(selected.sample_format(), SampleFormat::F32);
}

#[test]
fn capture_prefers_integer_precision_when_float_is_unavailable() {
    let configs = [
        range(SampleFormat::U8, 1, 48_000),
        range(SampleFormat::I16, 1, 48_000),
        range(SampleFormat::I32, 1, 48_000),
    ];

    let selected = select(configs, SampleRate(48_000), 1).unwrap();

    assert_eq!(selected.sample_format(), SampleFormat::I32);
}

#[test]
fn capture_selects_high_quality_multichannel_for_downmix() {
    let configs = [
        range(SampleFormat::U8, 2, 48_000),
        range(SampleFormat::F32, 2, 48_000),
    ];

    let selected = select(configs, SampleRate(48_000), 1).unwrap();

    assert_eq!(selected.sample_format(), SampleFormat::F32);
    assert_eq!(selected.channels(), 2);
}

#[test]
fn capture_skips_unsupported_sample_formats_and_rates() {
    let configs = [
        range(SampleFormat::I64, 1, 48_000),
        range(SampleFormat::F32, 1, 44_100),
        range(SampleFormat::I16, 1, 48_000),
    ];

    let selected = select(configs, SampleRate(48_000), 1).unwrap();

    assert_eq!(selected.sample_format(), SampleFormat::I16);
    assert_eq!(selected.sample_rate(), SampleRate(48_000));
}

#[test]
fn capture_retains_eight_bit_fallback_when_it_is_the_only_supported_format() {
    let selected = select([range(SampleFormat::U8, 1, 48_000)], SampleRate(48_000), 1).unwrap();

    assert_eq!(selected.sample_format(), SampleFormat::U8);
}

#[test]
fn capture_rejects_devices_without_supported_target_rate() {
    let selected = select([range(SampleFormat::F32, 1, 44_100)], SampleRate(48_000), 1);

    assert!(selected.is_none());
}

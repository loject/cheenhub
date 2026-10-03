//! Проверки сжатия времени входящего голоса без сдвига тона.

use super::*;

const TEST_SAMPLE_RATE_HZ: u32 = 48_000;
const FRAME_SAMPLES: usize = 960;

#[test]
fn passes_through_without_compression() {
    let mut compressor = VoiceTimeCompressor::new(TEST_SAMPLE_RATE_HZ);
    let frame = sine(FRAME_SAMPLES, 200.0, 0.0);

    let first = compressor.process(&frame);
    let second = compressor.process(&frame);

    assert_eq!(first, frame);
    assert_eq!(second, frame);
    assert_eq!(compressor.rate(), 1.0);
}

#[test]
fn passes_through_empty_frame() {
    let mut compressor = VoiceTimeCompressor::new(TEST_SAMPLE_RATE_HZ);

    assert!(compressor.process(&[]).is_empty());
}

#[test]
fn flush_releases_tail_once_and_does_not_leak_into_next_utterance() {
    let mut compressor = VoiceTimeCompressor::new(TEST_SAMPLE_RATE_HZ);
    compressor.process(&vec![0.2; FRAME_SAMPLES]);
    compressor.set_rate(1.2);
    for _ in 0..10 {
        compressor.process(&vec![0.2; FRAME_SAMPLES]);
    }
    let pending = compressor.pending_content_samples();
    assert!(pending > 0);
    let tail = compressor.flush();
    assert_eq!(tail.len(), pending);
    assert!(tail.iter().all(|sample| (*sample - 0.2).abs() < 1e-6));
    assert_eq!(compressor.pending_content_samples(), 0);
    assert_eq!(compressor.rate(), 1.0);
    assert!(compressor.flush().is_empty());
    assert_eq!(
        compressor.process(&vec![0.8; FRAME_SAMPLES]),
        vec![0.8; FRAME_SAMPLES]
    );
}

#[test]
fn flush_preserves_short_fragment_without_replaying_priming_history() {
    let mut compressor = VoiceTimeCompressor::new(TEST_SAMPLE_RATE_HZ);
    compressor.process(&vec![0.2; FRAME_SAMPLES]);
    compressor.set_rate(1.1);
    assert!(compressor.process(&[0.8; 24]).is_empty());
    assert_eq!(compressor.flush(), vec![0.8; 24]);
}

#[test]
fn flush_without_compression_does_not_replay_last_frame() {
    let mut compressor = VoiceTimeCompressor::new(TEST_SAMPLE_RATE_HZ);
    compressor.process(&vec![0.2; FRAME_SAMPLES]);
    assert!(compressor.flush().is_empty());
}

#[test]
fn shortens_signal_by_requested_rate() {
    let rate = 1.05;
    let mut compressor = VoiceTimeCompressor::new(TEST_SAMPLE_RATE_HZ);
    compressor.set_rate(rate);

    let mut input_len = 0;
    let mut output_len = 0;
    for frame_index in 0..50 {
        let frame = sine(FRAME_SAMPLES, 220.0, frame_index as f32 * 0.04);
        input_len += frame.len();
        output_len += compressor.process(&frame).len();
    }

    let expected = f64::from(input_len as u32) / rate;
    let deviation = (output_len as f64 - expected).abs();
    assert!(
        deviation < expected * 0.02,
        "output_len={output_len}, expected≈{expected}"
    );
}

#[test]
fn preserves_pitch_of_steady_tone() {
    let frequency_hz = 200.0;
    let source = sine(TEST_SAMPLE_RATE_HZ as usize, frequency_hz, 0.0);
    let output = compress(&source, 1.05);

    let measured_hz = estimate_frequency_hz(&output);
    assert!(
        (measured_hz - frequency_hz).abs() < frequency_hz * 0.01,
        "measured_hz={measured_hz}"
    );
    let residual = tone_residual(&output, frequency_hz);
    assert!(residual < 0.05, "residual={residual}");
}

#[test]
fn preserves_harmonic_structure_of_voice_like_tone() {
    let fundamental_hz = 150.0;
    let source = voice_like(TEST_SAMPLE_RATE_HZ as usize, fundamental_hz);
    let source_ratio = harmonic_ratio(&source, fundamental_hz);

    for rate in [1.02, 1.05, 1.1, MAX_TIME_COMPRESSION] {
        let output = compress(&source, rate);
        let output_ratio = harmonic_ratio(&output, fundamental_hz);
        assert!(
            output_ratio > source_ratio * 0.9,
            "rate={rate}: source_ratio={source_ratio}, output_ratio={output_ratio}"
        );
    }
}

#[test]
fn reports_pending_content_while_compressing() {
    let mut compressor = VoiceTimeCompressor::new(TEST_SAMPLE_RATE_HZ);
    compressor.set_rate(1.05);
    for frame_index in 0..10 {
        compressor.process(&sine(FRAME_SAMPLES, 200.0, frame_index as f32 * 0.04));
    }

    let pending = compressor.pending_content_samples();
    assert!(pending > 0, "pending={pending}");
    assert!(pending <= compressor.window_len * 4, "pending={pending}");
}

#[test]
fn keeps_steady_level_while_compressing() {
    let mut compressor = VoiceTimeCompressor::new(TEST_SAMPLE_RATE_HZ);
    compressor.set_rate(1.05);

    let mut output = Vec::new();
    for _ in 0..20 {
        output.extend_from_slice(&compressor.process(&vec![0.5_f32; FRAME_SAMPLES]));
    }

    let tail = &output[output.len() - FRAME_SAMPLES..];
    let max_deviation = tail.iter().fold(0.0_f32, |deviation, sample| {
        deviation.max((sample - 0.5).abs())
    });
    assert!(max_deviation < 0.02, "max_deviation={max_deviation}");
}

#[test]
fn flushes_pending_input_when_compression_stops() {
    let mut compressor = VoiceTimeCompressor::new(TEST_SAMPLE_RATE_HZ);
    let mut input_len = 0;
    let mut output_len = 0;
    for frame_index in 0..10 {
        compressor.set_rate(1.05);
        let frame = sine(FRAME_SAMPLES, 300.0, frame_index as f32 * 0.04);
        input_len += frame.len();
        output_len += compressor.process(&frame).len();
    }

    compressor.set_rate(1.0);
    for frame_index in 10..30 {
        let frame = sine(FRAME_SAMPLES, 300.0, frame_index as f32 * 0.04);
        input_len += frame.len();
        output_len += compressor.process(&frame).len();
    }

    let expected = f64::from(input_len as u32) / 1.05;
    let deviation = (output_len as f64 - expected).abs();
    assert!(
        deviation < expected * 0.05,
        "output_len={output_len}, expected≈{expected}"
    );
}

#[test]
fn caps_input_buffer_when_compression_cannot_drain() {
    let mut compressor = VoiceTimeCompressor::new(TEST_SAMPLE_RATE_HZ);
    compressor.set_rate(1.05);
    let mut output_len = 0;
    for frame_index in 0..200 {
        let frame = sine(FRAME_SAMPLES, 200.0, frame_index as f32 * 0.04);
        let output = compressor.process(&frame);
        assert!(output.iter().all(|sample| sample.is_finite()));
        output_len += output.len();
        assert!(
            compressor.input.len() <= compressor.window_len * MAX_INPUT_WINDOWS + FRAME_SAMPLES,
            "input_len={}",
            compressor.input.len()
        );
    }

    assert!(output_len > 0);
}

#[test]
fn recompresses_after_stopping() {
    let mut compressor = VoiceTimeCompressor::new(TEST_SAMPLE_RATE_HZ);
    compressor.set_rate(1.05);
    compressor.process(&sine(FRAME_SAMPLES, 250.0, 0.0));
    compressor.set_rate(1.0);
    compressor.process(&sine(FRAME_SAMPLES, 250.0, 0.02));

    compressor.set_rate(1.05);
    let output = compressor.process(&sine(FRAME_SAMPLES, 250.0, 0.04));

    assert!(!output.is_empty());
    assert!(output.iter().all(|sample| sample.is_finite()));
}

#[test]
fn clamps_requested_rate_to_supported_range() {
    let mut compressor = VoiceTimeCompressor::new(TEST_SAMPLE_RATE_HZ);

    compressor.set_rate(4.0);
    assert_eq!(compressor.rate(), MAX_TIME_COMPRESSION);

    compressor.set_rate(f64::NAN);
    assert_eq!(compressor.rate(), 1.0);

    compressor.set_rate(0.1);
    assert_eq!(compressor.rate(), 1.0);
}

#[test]
fn calculates_catch_up_rate_from_excess() {
    assert_eq!(catch_up_rate(0.0), 1.0);
    assert_eq!(catch_up_rate(-0.5), 1.0);
    assert_eq!(catch_up_rate(f64::NAN), 1.0);
    assert_eq!(catch_up_rate(0.001), 1.0);
    assert!((catch_up_rate(0.005) - 1.02).abs() < 1e-9);
    assert!((catch_up_rate(0.02) - 1.02).abs() < 1e-9);
    assert!((catch_up_rate(0.11) - 1.065).abs() < 1e-9);
    assert_eq!(catch_up_rate(0.2), MAX_TIME_COMPRESSION);
    assert_eq!(catch_up_rate(10.0), MAX_TIME_COMPRESSION);
}

#[test]
fn catch_up_rate_grows_faster_than_excess() {
    // Нелинейный закон: в зоне разгона наклон растёт с отставанием.
    let mut previous_slope = 0.0_f64;
    for step in 21..=200 {
        let excess = f64::from(step) * 0.001;
        let previous_excess = excess - 0.001;
        let slope = catch_up_rate(excess) - catch_up_rate(previous_excess);
        assert!(
            slope > previous_slope,
            "excess={excess}: slope={slope}, previous={previous_slope}"
        );
        previous_slope = slope;
    }
}

#[test]
fn catch_up_rate_stays_within_supported_range() {
    for step in 0..=200 {
        let rate = catch_up_rate(f64::from(step) * 0.01);
        assert!((1.0..=MAX_TIME_COMPRESSION).contains(&rate), "rate={rate}");
    }
}

/// Прогоняет сигнал через компрессор фреймами по 20 мс.
fn compress(source: &[f32], rate: f64) -> Vec<f32> {
    let mut compressor = VoiceTimeCompressor::new(TEST_SAMPLE_RATE_HZ);
    compressor.set_rate(rate);
    let mut output = Vec::with_capacity(source.len());
    for frame in source.chunks(FRAME_SAMPLES) {
        output.extend_from_slice(&compressor.process(frame));
    }
    output
}

fn sine(len: usize, frequency_hz: f32, start_seconds: f32) -> Vec<f32> {
    let sample_rate = TEST_SAMPLE_RATE_HZ as f32;
    (0..len)
        .map(|index| {
            let seconds = start_seconds + index as f32 / sample_rate;
            (2.0 * PI * f64::from(frequency_hz) * f64::from(seconds)).sin() as f32
        })
        .collect()
}

/// Сумма гармоник с убывающей амплитудой — упрощённый гласный звук.
fn voice_like(len: usize, fundamental_hz: f32) -> Vec<f32> {
    const AMPLITUDES: [f64; 6] = [1.0, 0.6, 0.4, 0.25, 0.15, 0.1];
    (0..len)
        .map(|index| {
            let seconds = index as f64 / f64::from(TEST_SAMPLE_RATE_HZ);
            AMPLITUDES
                .iter()
                .enumerate()
                .map(|(harmonic, amplitude)| {
                    amplitude
                        * (2.0 * PI * f64::from(fundamental_hz) * (harmonic + 1) as f64 * seconds)
                            .sin()
                })
                .sum::<f64>() as f32
                * 0.2
        })
        .collect()
}

/// Доля энергии на частотах гармоник относительно всей энергии сигнала.
fn harmonic_ratio(samples: &[f32], fundamental_hz: f32) -> f32 {
    const HARMONIC_COUNT: usize = 8;
    const ANALYSIS_SAMPLES: usize = 4_096;
    if samples.len() <= ANALYSIS_SAMPLES + 1 {
        return 0.0;
    }
    let start = (samples.len() - ANALYSIS_SAMPLES) / 2;
    let segment = &samples[start..start + ANALYSIS_SAMPLES];
    let mut harmonic_energy = 0.0_f64;
    let mut inter_harmonic_energy = 0.0_f64;
    for harmonic in 0..HARMONIC_COUNT {
        let frequency_hz = f64::from(fundamental_hz) * (harmonic + 1) as f64;
        harmonic_energy += goertzel_energy(segment, frequency_hz);
        inter_harmonic_energy += goertzel_energy(segment, frequency_hz * 1.08);
    }

    (harmonic_energy / (harmonic_energy + inter_harmonic_energy).max(f64::MIN_POSITIVE)) as f32
}

/// Энергия компоненты сигнала на заданной частоте.
fn goertzel_energy(samples: &[f32], frequency_hz: f64) -> f64 {
    let omega = 2.0 * PI * frequency_hz / f64::from(TEST_SAMPLE_RATE_HZ);
    let mut real = 0.0_f64;
    let mut imaginary = 0.0_f64;
    for (index, sample) in samples.iter().enumerate() {
        let phase = omega * index as f64;
        real += f64::from(*sample) * phase.cos();
        imaginary += f64::from(*sample) * phase.sin();
    }
    real * real + imaginary * imaginary
}

/// Доля энергии, не объяснённая чистым тоном заданной частоты.
fn tone_residual(samples: &[f32], frequency_hz: f32) -> f32 {
    const ANALYSIS_SAMPLES: usize = 8_192;
    if samples.len() <= ANALYSIS_SAMPLES + 1 {
        return 1.0;
    }
    let start = (samples.len() - ANALYSIS_SAMPLES) / 2;
    let segment = &samples[start..start + ANALYSIS_SAMPLES];
    let omega = 2.0 * PI * f64::from(frequency_hz) / f64::from(TEST_SAMPLE_RATE_HZ);
    let mut sin_energy = 0.0_f64;
    let mut cos_energy = 0.0_f64;
    let mut cross_energy = 0.0_f64;
    let mut signal_cos = 0.0_f64;
    let mut signal_sin = 0.0_f64;
    let mut total_energy = 0.0_f64;
    for (index, sample) in segment.iter().enumerate() {
        let phase = omega * index as f64;
        let (sin_phase, cos_phase) = phase.sin_cos();
        let sample = f64::from(*sample);
        sin_energy += sin_phase * sin_phase;
        cos_energy += cos_phase * cos_phase;
        cross_energy += sin_phase * cos_phase;
        signal_sin += sample * sin_phase;
        signal_cos += sample * cos_phase;
        total_energy += sample * sample;
    }
    let determinant = sin_energy * cos_energy - cross_energy * cross_energy;
    let fitted_energy = (signal_sin * signal_sin * cos_energy
        - 2.0 * signal_sin * signal_cos * cross_energy
        + signal_cos * signal_cos * sin_energy)
        / determinant;
    ((total_energy - fitted_energy) / total_energy)
        .max(0.0)
        .sqrt() as f32
}

fn estimate_frequency_hz(samples: &[f32]) -> f32 {
    let crossings = samples
        .windows(2)
        .filter(|pair| pair[0] <= 0.0 && pair[1] > 0.0)
        .count();
    crossings as f32 * TEST_SAMPLE_RATE_HZ as f32 / samples.len() as f32
}

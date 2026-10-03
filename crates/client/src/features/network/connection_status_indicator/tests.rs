use super::{PingSample, graph_points};

#[test]
fn graph_handles_a_minute_of_voice_frequency_samples() {
    let samples = (0..=80)
        .map(|index| PingSample {
            received_at_ms: index * 750,
            rtt_ms: 20.0 + (index % 7) as f64,
            jitter_ms: 1.0,
        })
        .collect::<Vec<_>>();

    let points = graph_points(&samples);

    assert_eq!(points.split_whitespace().count(), samples.len());
    assert!(!points.contains("NaN"));
    assert!(!points.contains("inf"));
}

#[test]
fn graph_handles_equal_sample_timestamps() {
    let samples = vec![
        PingSample {
            received_at_ms: 1_000,
            rtt_ms: 20.0,
            jitter_ms: 0.0,
        },
        PingSample {
            received_at_ms: 1_000,
            rtt_ms: 24.0,
            jitter_ms: 4.0,
        },
    ];

    let points = graph_points(&samples);

    assert_eq!(points.split_whitespace().count(), 2);
    assert!(!points.contains("NaN"));
    assert!(!points.contains("inf"));
}

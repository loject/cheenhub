use super::{HISTORY_WINDOW_MS, SAMPLE_GAP_MS, chart_option, nice_max, prune_to_window};
use cheenhub_contracts::rest::HostVoiceActivitySample;

fn sample(unix_ms: i64, voice: u32, video: u32) -> HostVoiceActivitySample {
    HostVoiceActivitySample {
        sampled_at_unix_ms: unix_ms,
        voice_connections: voice,
        video_sources: video,
    }
}

fn series_values(option: &str, index: usize) -> Vec<serde_json::Value> {
    let option: serde_json::Value =
        serde_json::from_str(option).expect("chart option is valid JSON");
    option["series"][index]["data"]
        .as_array()
        .expect("series data is an array")
        .clone()
}

#[test]
fn measurement_gap_becomes_a_break_instead_of_a_drop_to_zero() {
    let samples = vec![
        sample(1_000, 0, 0),
        sample(1_010, 4, 2),
        sample(1_010 + SAMPLE_GAP_MS * 2, 2, 1),
    ];

    let option = chart_option(&samples);
    let voice = series_values(&option, 0);

    assert_eq!(voice.len(), 4, "пропуск добавляет одну пустую точку");
    assert_eq!(voice[2][1], serde_json::Value::Null);
    assert_eq!(
        voice[3][1], 2,
        "после простоя линия продолжается с реального значения"
    );
}

#[test]
fn both_series_use_real_timestamps_and_own_values() {
    let samples = vec![sample(1_000, 1, 6), sample(11_000, 5, 3)];

    let option = chart_option(&samples);
    let voice = series_values(&option, 0);
    let video = series_values(&option, 1);

    assert_eq!(voice[0][0], 1_000);
    assert_eq!(video[1][1], 3);
    let parsed: serde_json::Value =
        serde_json::from_str(&option).expect("chart option is valid JSON");
    assert_eq!(parsed["xAxis"]["type"], "time");
    assert_eq!(parsed["series"][0]["smooth"], 0.25);
    assert_eq!(parsed["series"][0]["connectNulls"], false);
}

#[test]
fn axis_top_is_rounded_up_for_whole_numbers() {
    assert_eq!(nice_max(0), 4);
    assert_eq!(nice_max(3), 4);
    assert_eq!(nice_max(7), 8);
    assert_eq!(nice_max(23), 25);

    let option = chart_option(&[sample(0, 23, 2)]);
    let parsed: serde_json::Value =
        serde_json::from_str(&option).expect("chart option is valid JSON");
    assert_eq!(parsed["yAxis"]["max"], 25);
}

#[test]
fn window_pruning_keeps_only_last_24_hours() {
    let newest = HISTORY_WINDOW_MS * 3;
    let samples = vec![
        sample(newest - HISTORY_WINDOW_MS - 1_000, 1, 0),
        sample(newest - HISTORY_WINDOW_MS / 2, 3, 2),
        sample(newest, 1, 4),
    ];

    assert_eq!(
        prune_to_window(samples)
            .iter()
            .map(|sample| sample.voice_connections)
            .collect::<Vec<_>>(),
        vec![3, 1]
    );
}

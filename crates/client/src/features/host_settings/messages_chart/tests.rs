use super::{MINUTE_MS, MessagesChartData, chart_option, nice_max, prune_to_window};
use cheenhub_contracts::rest::HostMessagesPerMinuteSample;

fn sample(minute_offset: i64, messages: u64) -> HostMessagesPerMinuteSample {
    HostMessagesPerMinuteSample {
        minute_unix_ms: minute_offset * MINUTE_MS,
        messages,
    }
}

fn data(
    rooms: Vec<HostMessagesPerMinuteSample>,
    direct: Vec<HostMessagesPerMinuteSample>,
) -> MessagesChartData {
    MessagesChartData {
        window_end_unix_ms: rooms
            .iter()
            .chain(&direct)
            .map(|sample| sample.minute_unix_ms)
            .max()
            .unwrap_or(0),
        room_samples: rooms,
        direct_samples: direct,
    }
}

fn parsed(option: &str) -> serde_json::Value {
    serde_json::from_str(option).expect("chart option is valid JSON")
}

fn series_values(option: &str, index: usize) -> Vec<serde_json::Value> {
    parsed(option)["series"][index]["data"]
        .as_array()
        .expect("series data is an array")
        .clone()
}

fn series_name(option: &str, index: usize) -> String {
    parsed(option)["series"][index]["name"]
        .as_str()
        .expect("series has a name")
        .to_owned()
}

#[test]
fn bars_keep_minute_timestamps_and_message_counts() {
    let option = chart_option(&data(vec![sample(0, 1), sample(1, 7)], vec![sample(1, 2)]));

    let rooms = series_values(&option, 0);
    assert_eq!(rooms[0][0], 0);
    assert_eq!(rooms[1][0], MINUTE_MS);
    assert_eq!(rooms[1][1], 7);
}

#[test]
fn direct_messages_are_drawn_as_a_separate_series() {
    let option = chart_option(&data(vec![sample(0, 1)], vec![sample(0, 2)]));

    assert_eq!(series_name(&option, 0), "В комнатах");
    assert_eq!(series_name(&option, 1), "Личные");
    assert_eq!(
        series_values(&option, 1)[0][1],
        2,
        "личные сообщения не смешиваются с сообщениями комнат"
    );
}

#[test]
fn quiet_host_still_gets_readable_axis_and_empty_series() {
    let option = chart_option(&data(Vec::new(), Vec::new()));

    assert_eq!(
        parsed(&option)["yAxis"]["max"],
        4,
        "шкала остаётся читаемой без данных"
    );
    assert!(series_values(&option, 0).is_empty());
    assert!(series_values(&option, 1).is_empty());
    assert_eq!(parsed(&option)["xAxis"]["type"], "time");
}

#[test]
fn axis_top_is_rounded_up_and_covers_both_series() {
    assert_eq!(nice_max(0), 4);
    assert_eq!(nice_max(3), 4);
    assert_eq!(nice_max(7), 8);
    assert_eq!(nice_max(23), 25);

    let option = chart_option(&data(vec![sample(0, 3)], vec![sample(0, 23)]));

    assert_eq!(
        parsed(&option)["yAxis"]["max"],
        25,
        "шкала строится по пику обеих серий"
    );
}

#[test]
fn window_pruning_shares_one_day_window_between_both_series() {
    let newest_minute = 60 * 24 * 3;
    let pruned = prune_to_window(data(
        vec![
            sample(newest_minute - 24 * 60 - 1, 5),
            sample(newest_minute - 60, 1),
        ],
        vec![sample(newest_minute, 3)],
    ));

    assert_eq!(
        pruned.room_samples.len(),
        1,
        "старые минуты не попадают в график"
    );
    assert_eq!(pruned.room_samples[0].messages, 1);
    assert_eq!(pruned.direct_samples.len(), 1);
    assert_eq!(pruned.direct_samples[0].messages, 3);
}

#[test]
fn window_pruning_uses_the_freshest_minute_of_the_slow_series() {
    // Комнаты молчали дольше суток, личные диалоги — нет: окно одно, поэтому
    // старые комнатные минуты исчезают вместе с ними.
    let pruned = prune_to_window(data(vec![sample(0, 5)], vec![sample(60 * 30, 3)]));

    assert!(pruned.room_samples.is_empty());
    assert_eq!(pruned.direct_samples.len(), 1);
}

#[test]
fn sparse_messages_keep_a_full_day_axis() {
    let option = parsed(&chart_option(&data(vec![sample(600, 2)], Vec::new())));

    let start = option["xAxis"]["min"]
        .as_i64()
        .expect("explicit window start");
    let end = option["xAxis"]["max"]
        .as_i64()
        .expect("explicit window end");
    assert_eq!(end - start, 86_400_000);
}

#[test]
fn sparse_messages_use_snapshot_time_instead_of_the_last_message() {
    let mut samples = data(vec![sample(600, 2)], Vec::new());
    samples.window_end_unix_ms = 1_800 * MINUTE_MS;

    let option = parsed(&chart_option(&samples));

    assert_eq!(option["xAxis"]["min"], 21_600_000);
    assert_eq!(option["xAxis"]["max"], 108_000_000);
}

#[test]
fn window_pruning_removes_old_messages_after_a_quiet_day() {
    let mut samples = data(vec![sample(0, 2)], Vec::new());
    samples.window_end_unix_ms = 1_800 * MINUTE_MS;

    let pruned = prune_to_window(samples);

    assert!(pruned.is_empty());
}

#[test]
fn window_pruning_keeps_the_partial_first_minute() {
    let mut samples = data(vec![sample(360, 2)], Vec::new());
    samples.window_end_unix_ms = 108_030_000;

    let pruned = prune_to_window(samples);

    assert_eq!(pruned.room_samples.len(), 1);
    assert_eq!(pruned.room_samples[0].messages, 2);
}

#[test]
fn partial_first_minute_is_drawn_inside_the_axis_window() {
    let mut samples = data(vec![sample(360, 2)], Vec::new());
    samples.window_end_unix_ms = 108_030_000;

    let option = parsed(&chart_option(&samples));

    assert_eq!(option["series"][0]["data"][0][0], 21_630_000);
    assert_eq!(option["series"][0]["data"][0][1], 2);
}

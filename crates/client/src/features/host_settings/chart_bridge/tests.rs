use super::{bridge_script, new_chart_id};

#[test]
fn bridge_script_substitutes_vendored_library_url() {
    let script = bridge_script();

    assert!(
        !script.contains("__ECHARTS_URL__"),
        "адрес библиотеки подставляется в скрипт моста"
    );
    assert!(script.contains("/vendor/echarts.min.js"));
}

#[test]
fn chart_ids_are_unique_per_graph_kind() {
    let first = new_chart_id("messages");
    let second = new_chart_id("messages");
    let activity = new_chart_id("activity");

    assert_ne!(first, second, "каждый график получает свой контейнер");
    assert!(first.contains("messages") && activity.contains("activity"));
}

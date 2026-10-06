use super::{cpu_value, disk_hint, disk_value, memory_hint, memory_value, network_value};

#[test]
fn summary_cards_degrade_to_placeholders_without_samples() {
    assert_eq!(cpu_value(None), "—");
    assert_eq!(memory_value(None), "—");
    assert_eq!(disk_value(None), "—");
    assert_eq!(memory_hint(None), "нет данных");
    assert_eq!(disk_hint(None), "нет данных");
    assert_eq!(
        network_value(None, |sample| sample.network.sent_bytes_per_second),
        "—"
    );
}

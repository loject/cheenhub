//! Проверки подписей и реального перечисления PulseAudio.

use super::*;

#[test]
fn duplicate_descriptions_keep_distinct_stable_identifiers() {
    let devices = disambiguate(vec![
        AudioInputDevice {
            device_id: "usb-b".into(),
            label: "Headset".into(),
        },
        AudioInputDevice {
            device_id: "usb-a".into(),
            label: "Headset".into(),
        },
        AudioInputDevice {
            device_id: "internal".into(),
            label: "Speakers".into(),
        },
    ]);
    assert_eq!(devices.len(), 3);
    assert_eq!(
        (devices[0].device_id.as_str(), devices[0].label.as_str()),
        ("usb-a", "Headset (1)")
    );
    assert_eq!(
        (devices[1].device_id.as_str(), devices[1].label.as_str()),
        ("usb-b", "Headset (2)")
    );
    assert_eq!(devices[2].label, "Speakers");
}

#[test]
#[ignore = "требуется доступ к пользовательскому PulseAudio или PipeWire Pulse"]
fn live_pulse_enumeration() {
    let devices = enumerate_blocking().expect("PulseAudio enumeration must succeed");
    for device in &devices {
        println!("{}: {}", device.device_id, device.label);
    }
    assert!(
        devices
            .iter()
            .all(|device| !device.device_id.is_empty() && !device.label.is_empty())
    );
}

#[test]
fn monitor_sources_are_excluded_and_descriptions_are_separate_from_ids() {
    assert!(input_device(Some("monitor"), Some("Output monitor"), Some(0)).is_none());
    assert!(input_device(None, Some("Microphone"), None).is_none());
    let device = input_device(Some("alsa_input.usb"), Some("USB Microphone"), None).unwrap();
    assert_eq!(device.device_id, "alsa_input.usb");
    assert_eq!(device.label, "USB Microphone");
    assert_eq!(
        input_device(Some("unnamed"), Some(" "), None)
            .unwrap()
            .label,
        "Микрофон"
    );
}

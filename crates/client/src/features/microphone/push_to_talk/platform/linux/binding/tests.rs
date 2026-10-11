use super::*;

#[test]
fn portal_metadata_roundtrip_ignores_windows_virtual_keys() {
    let _guard = TEST_LOCK.lock().unwrap();
    assert_eq!(PushToTalkKey::from_value("163"), PushToTalkKey::default());
    let binding = PushToTalkKey::from_value("portal:7:Ctrl + Space");

    let saved = binding.storage_value();
    let restored = PushToTalkKey::from_value(&saved);

    assert_eq!(restored, binding);
    assert_eq!(restored.label(), "Ctrl + Space");
    assert_ne!(PushToTalkKey::assigned(), binding);
}

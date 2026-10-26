//! Проверка сохранения mute при временно остановленном native-захвате.

use crate::features::{
    microphone::{MicrophoneHandle, MicrophoneStatus},
    realtime::create_handle,
};
use dioxus::prelude::*;
use dioxus_sdk_storage::{LocalStorage, StorageBacking};

#[test]
fn toggling_idle_enabled_microphone_saves_mute_for_next_application_launch() {
    let directory = crate::tests::native::storage_directory();
    let dom = VirtualDom::new(|| rsx! {});
    dom.in_scope(ScopeId::ROOT, || {
        let microphone = MicrophoneHandle::for_test();
        assert_eq!(microphone.status(), MicrophoneStatus::Idle);
        assert!(microphone.voice_enabled());

        crate::features::voice_chat::microphone_uplink::toggle(
            microphone.clone(),
            create_handle(),
            uuid::Uuid::new_v4().to_string(),
            uuid::Uuid::new_v4().to_string(),
        );

        assert!(!microphone.voice_enabled());
        assert_eq!(
            LocalStorage::get::<Option<String>>(&"cheenhub.microphone.voice_enabled".to_owned()),
            Some(Some("false".to_owned()))
        );
    });
    std::fs::remove_dir_all(directory).unwrap();
}

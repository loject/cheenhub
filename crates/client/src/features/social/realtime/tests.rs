use cheenhub_contracts::{
    presence::UserPresence,
    realtime::{PresenceChanged, RealtimeEnvelope, RealtimeKind, RealtimeModule, SocialKind},
};

use super::decode_presence_event;

#[test]
fn disconnected_social_list_returns_readable_error() {
    use dioxus::prelude::*;
    use futures_util::FutureExt;

    let dom = VirtualDom::new(|| rsx! {});
    dom.in_scope(ScopeId::ROOT, || {
        let realtime = crate::features::realtime::create_handle();
        let result = super::list_social_entries(&realtime, None, None)
            .now_or_never()
            .expect("отключённый запрос завершается сразу");
        assert_eq!(
            result.unwrap_err(),
            "Не удалось загрузить список друзей и диалогов. Проверь соединение и повтори попытку."
        );
    });
}

#[test]
fn presence_event_decodes_from_social_stream_and_keeps_last_seen() {
    let payload = PresenceChanged {
        user_id: "friend".to_owned(),
        presence: UserPresence::Offline {
            last_seen_at: Some("2026-10-28T12:30:00Z".to_owned()),
        },
    };
    let envelope = RealtimeEnvelope::new(
        RealtimeModule::Social,
        RealtimeKind::Social(SocialKind::PresenceChanged),
        None,
        payload.clone(),
    )
    .unwrap();

    assert_eq!(decode_presence_event(envelope), Some(payload));

    let other = RealtimeEnvelope::new(
        RealtimeModule::Network,
        RealtimeKind::Network(cheenhub_contracts::realtime::NetworkKind::Ping),
        None,
        cheenhub_contracts::realtime::Ping { sent_at_ms: 1 },
    )
    .unwrap();
    assert_eq!(decode_presence_event(other), None);
}

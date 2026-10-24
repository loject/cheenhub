use cheenhub_contracts::realtime::RealtimeModule;

use super::{ReliableRequestMode, default_request_mode, new_request_id};

#[test]
fn rejected_response_exposes_server_message_for_the_user() {
    use cheenhub_contracts::realtime::{
        ControlKind, RealtimeEnvelope, RealtimeKind, Rejected, RejectionCode,
    };
    let response = RealtimeEnvelope::new(
        RealtimeModule::Control,
        RealtimeKind::Control(ControlKind::Rejected),
        Some(uuid::Uuid::new_v4()),
        Rejected {
            code: RejectionCode::BadRequest,
            message: "Писать можно только друзьям.".to_owned(),
        },
    )
    .unwrap();

    let error = super::decode_response::<()>(response).unwrap_err();

    assert_eq!(error.user_message(), Some("Писать можно только друзьям."));
}

#[test]
fn retry_attempt_uses_a_new_request_id() {
    assert_ne!(new_request_id(), new_request_id());
}

#[test]
fn server_requests_default_to_one_shot() {
    assert!(default_request_mode(RealtimeModule::Server) == ReliableRequestMode::OneShot);
}

#[test]
fn cached_modules_default_to_cached_stream() {
    assert!(default_request_mode(RealtimeModule::TextChat) == ReliableRequestMode::Cached);
    assert!(default_request_mode(RealtimeModule::Social) == ReliableRequestMode::Cached);
}

#[test]
fn cached_request_routes_event_before_matching_response() {
    use super::{ConnectedSession, ConnectedTransport, create_handle};
    use cheenhub_contracts::realtime::{
        RealtimeEnvelope, RealtimeKind, SocialChangeReason, SocialChanged, SocialKind, SocialReady,
        SubscribeSocial,
    };
    use dioxus::prelude::*;
    use futures_util::{FutureExt, StreamExt};

    let mut dom = VirtualDom::new(|| rsx! {});
    let (mut request, mut events) = dom.in_scope(ScopeId::ROOT, || {
        let realtime = create_handle();
        let (sender, mut outbound) = futures_channel::mpsc::unbounded();
        let (sender, _, _) =
            crate::features::realtime::websocket::WebSocketOutboundSender::new(sender);
        *realtime.inner.session.try_lock().unwrap() = Some(ConnectedSession {
            generation: 1,
            transport: ConnectedTransport::WebSocket(sender),
        });
        let events = realtime.subscribe_events();
        let requester = realtime.clone();
        let mut request = async move {
            requester
                .request::<_, SocialReady>(
                    RealtimeModule::Social,
                    RealtimeKind::Social(SocialKind::Subscribe),
                    SubscribeSocial,
                )
                .await
        }
        .boxed_local();
        assert!(request.as_mut().now_or_never().is_none());
        let Some(Some(super::WebSocketOutbound::Envelope(sent))) = outbound.next().now_or_never()
        else {
            panic!("request should reach the writer");
        };
        let changed = RealtimeEnvelope::new(
            RealtimeModule::Social,
            RealtimeKind::Social(SocialKind::Changed),
            None,
            SocialChanged {
                reason: SocialChangeReason::DirectMessages,
                conversation_id: None,
            },
        )
        .unwrap();
        let ready = RealtimeEnvelope::new(
            RealtimeModule::Social,
            RealtimeKind::Social(SocialKind::Ready),
            sent.request_id,
            SocialReady,
        )
        .unwrap();

        realtime.inner.inbound.unbounded_send(changed).unwrap();
        realtime.inner.inbound.unbounded_send(ready).unwrap();
        (request, events)
    });

    dom.render_immediate_to_vec();

    assert!(request.as_mut().now_or_never().unwrap().is_ok());
    assert_eq!(
        events.next().now_or_never().unwrap().unwrap().kind,
        RealtimeKind::Social(SocialKind::Changed)
    );
}

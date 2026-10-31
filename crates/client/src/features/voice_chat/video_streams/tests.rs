//! Регрессии времени жизни задач, обслуживающих активные видеопотоки.

use super::*;
use dioxus::logger::tracing::field::{Field, Visit};
use dioxus::logger::tracing::span::{Attributes, Id, Record};
use dioxus::logger::tracing::{self, Event, Metadata, Subscriber};
use std::rc::Weak;
use std::sync::{Arc, Mutex};

struct SignalWarnings(Arc<Mutex<Vec<String>>>);

impl Subscriber for SignalWarnings {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        *metadata.level() == tracing::Level::WARN
    }
    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }
    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}
    fn event(&self, event: &Event<'_>) {
        struct Message(String);
        impl Visit for Message {
            fn record_debug(&mut self, _: &Field, value: &dyn std::fmt::Debug) {
                self.0.push_str(&format!("{value:?}"));
            }
        }
        let mut message = Message(String::new());
        event.record(&mut message);
        if message.0.contains("A Copy Value created") {
            self.0.lock().unwrap().push(message.0);
        }
    }
}

type WeakGenerations = Weak<RefCell<HashMap<ParticipantVideoKey, u64>>>;

#[derive(Clone)]
struct VideoOwnerScenario {
    visible: Signal<bool>,
    generations: Rc<RefCell<WeakGenerations>>,
}

fn scenario_app() -> Element {
    let scenario = use_context::<VideoOwnerScenario>();
    if (scenario.visible)() {
        rsx! { VideoOwner {} }
    } else {
        rsx! { p { "Звонок закрыт" } }
    }
}

#[component]
fn VideoOwner() -> Element {
    let scenario = use_context::<VideoOwnerScenario>();
    let live_streams = use_signal(Vec::new);
    use_hook(move || {
        let generations = Rc::new(RefCell::new(HashMap::new()));
        *scenario.generations.borrow_mut() = Rc::downgrade(&generations);
        let handle = ParticipantVideoHandle::new(
            live_streams,
            Rc::new(RefCell::new(HashMap::new())),
            generations,
            Rc::new(RefCell::new(HashMap::new())),
            dioxus::core::current_scope_id(),
        );
        handle.mark_live(ParticipantVideoKey::new(
            ParticipantVideoSource::Camera,
            "peer".to_owned(),
            uuid::Uuid::from_u128(1),
        ));
        handle
    });
    rsx! { div {} }
}

fn video_handle(dom: &VirtualDom) -> ParticipantVideoHandle {
    dom.in_scope(ScopeId::ROOT, || {
        ParticipantVideoHandle::new(
            Signal::new(Vec::new()),
            Rc::new(RefCell::new(HashMap::new())),
            Rc::new(RefCell::new(HashMap::new())),
            Rc::new(RefCell::new(HashMap::new())),
            ScopeId::ROOT,
        )
    })
}

fn frame(sequence: u64, key_frame: bool) -> ParticipantVideoFrame {
    ParticipantVideoFrame {
        room_id: "room".into(),
        sender_user_id: "peer".into(),
        track_id: uuid::Uuid::from_u128(1),
        sequence,
        timestamp_us: sequence * 1_000_000,
        duration_us: 1_000_000,
        bytes: vec![1, 2, 3],
        key_frame,
    }
}

#[test]
fn video_tile_receives_initial_key_frame_and_following_deltas_after_mounting() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(1, false));
    });

    let mut frames = handle.subscribe_stream(
        ParticipantVideoSource::ScreenShare,
        "peer".into(),
        uuid::Uuid::from_u128(1),
    );

    assert_eq!(frames.try_recv().unwrap(), frame(0, true));
    assert_eq!(frames.try_recv().unwrap(), frame(1, false));
}

#[test]
fn static_screen_remains_visible_without_new_frames_until_explicit_stop() {
    let mut dom = VirtualDom::new(|| rsx! {});
    dom.rebuild_in_place();
    let handle = video_handle(&dom);
    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
    });

    dom.render_immediate_to_vec();

    dom.in_scope(ScopeId::ROOT, || {
        assert_eq!(
            handle.live_tracks(ParticipantVideoSource::ScreenShare),
            vec![("peer".to_owned(), uuid::Uuid::from_u128(1))]
        );
        handle.release_stream(
            ParticipantVideoSource::ScreenShare,
            "peer",
            uuid::Uuid::from_u128(1),
        );
        assert!(
            handle
                .live_tracks(ParticipantVideoSource::ScreenShare)
                .is_empty()
        );
    });
}

fn participant_session(
    user_id: &str,
    joined_at: &str,
) -> cheenhub_contracts::realtime::VoiceRoomParticipant {
    cheenhub_contracts::realtime::VoiceRoomParticipant {
        microphone_enabled: true,
        output_muted: false,
        user_id: user_id.to_owned(),
        session_id: uuid::Uuid::from_u128(1).to_string(),
        voice_protocol_version: 4,
        device_kind: cheenhub_contracts::realtime::VoiceDeviceKind::Desktop,
        nickname: "Участник".to_owned(),
        avatar_url: None,
        joined_at: joined_at.to_owned(),
    }
}

#[test]
fn departed_participant_releases_stale_screen_share() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.reconcile_participants(&[participant_session("peer", "2026-10-28T09:00:00Z")], &[]);
    });

    assert!(
        handle
            .live_tracks(ParticipantVideoSource::ScreenShare)
            .is_empty()
    );
}

#[test]
fn departed_session_cannot_resurrect_screen_share_from_late_key_frame() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.reconcile_participants(&[participant_session("peer", "2026-10-28T09:00:00Z")], &[]);
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(1, true));
    });

    assert!(
        handle
            .live_tracks(ParticipantVideoSource::ScreenShare)
            .is_empty(),
        "late key frame must not revive video for a departed voice session"
    );
}

#[test]
fn rejoined_voice_session_can_restart_video_with_a_fresh_key_frame() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.reconcile_participants(&[participant_session("peer", "2026-10-28T09:00:00Z")], &[]);
        handle.reconcile_participants(&[], &[participant_session("peer", "2026-10-28T09:01:00Z")]);
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(2, true));
    });

    assert_eq!(
        handle.live_tracks(ParticipantVideoSource::ScreenShare),
        vec![("peer".to_owned(), Uuid::from_u128(1))]
    );
}

#[test]
fn replaced_participant_session_releases_stale_screen_share() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.reconcile_participants(
            &[participant_session("peer", "2026-10-28T09:00:00Z")],
            &[cheenhub_contracts::realtime::VoiceRoomParticipant {
                session_id: uuid::Uuid::from_u128(2).to_string(),
                ..participant_session("peer", "2026-10-28T09:01:00Z")
            }],
        );
    });

    assert!(
        handle
            .live_tracks(ParticipantVideoSource::ScreenShare)
            .is_empty()
    );
}

#[test]
fn rejoining_same_voice_session_with_new_join_time_releases_stale_screen_share() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.reconcile_participants(
            &[participant_session("peer", "2026-10-28T09:00:00Z")],
            &[participant_session("peer", "2026-10-28T09:01:00Z")],
        );
    });

    assert!(
        handle
            .live_tracks(ParticipantVideoSource::ScreenShare)
            .is_empty()
    );
}

#[test]
fn unchanged_participant_session_keeps_static_screen_share() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.reconcile_participants(
            &[participant_session("peer", "2026-10-28T09:00:00Z")],
            &[participant_session("peer", "2026-10-28T09:00:00Z")],
        );
    });

    assert_eq!(
        handle.live_tracks(ParticipantVideoSource::ScreenShare),
        vec![("peer".to_owned(), uuid::Uuid::from_u128(1))],
    );
}

#[test]
fn explicit_stop_discards_buffered_frames_and_restart_waits_for_a_fresh_key_frame() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.release_stream(
            ParticipantVideoSource::ScreenShare,
            "peer",
            uuid::Uuid::from_u128(1),
        );
    });
    let mut receiver = handle.subscribe_stream(
        ParticipantVideoSource::ScreenShare,
        "peer".into(),
        uuid::Uuid::from_u128(1),
    );
    assert!(receiver.try_recv().is_err());

    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(1, false));
    });
    assert!(receiver.try_recv().is_err());
    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(2, true));
    });

    assert_eq!(receiver.try_recv().unwrap(), frame(2, true));
}

#[test]
fn legacy_video_stop_without_session_id_releases_only_users_source() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    let mut other_frame = frame(0, true);
    other_frame.sender_user_id = "other".to_owned();
    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.publish_frame(ParticipantVideoSource::Camera, frame(0, true));
        handle.publish_frame(ParticipantVideoSource::ScreenShare, other_frame);
        handle.release_stream_for_event(ParticipantVideoSource::ScreenShare, "peer", Uuid::nil());
    });

    assert_eq!(
        handle.live_tracks(ParticipantVideoSource::ScreenShare),
        vec![("other".to_owned(), Uuid::from_u128(1))]
    );
    assert_eq!(
        handle.live_tracks(ParticipantVideoSource::Camera),
        vec![("peer".to_owned(), Uuid::from_u128(1))]
    );
}

#[test]
fn legacy_participant_departure_releases_video_without_session_id() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    let mut legacy_participant = participant_session("peer", "2026-10-28T09:00:00Z");
    legacy_participant.session_id.clear();
    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.reconcile_participants(&[legacy_participant], &[]);
    });
    assert!(
        handle
            .live_tracks(ParticipantVideoSource::ScreenShare)
            .is_empty()
    );
}

#[test]
fn legacy_rejoin_with_changed_join_time_releases_stale_video() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    let mut previous = participant_session("peer", "2026-10-28T09:00:00Z");
    let mut current = participant_session("peer", "2026-10-28T09:01:00Z");
    previous.session_id.clear();
    current.session_id.clear();

    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.reconcile_participants(&[previous], &[current]);
    });

    assert!(
        handle
            .live_tracks(ParticipantVideoSource::ScreenShare)
            .is_empty()
    );
}

#[test]
fn legacy_presence_with_unchanged_join_time_keeps_static_video() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    let mut participant = participant_session("peer", "2026-10-28T09:00:00Z");
    participant.session_id.clear();

    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.reconcile_participants(&[participant.clone()], &[participant]);
    });

    assert_eq!(
        handle.live_tracks(ParticipantVideoSource::ScreenShare),
        vec![("peer".to_owned(), Uuid::from_u128(1))]
    );
}

#[test]
fn leaving_the_call_discards_buffered_screen_frames() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.clear();
    });

    let mut receiver = handle.subscribe_stream(
        ParticipantVideoSource::ScreenShare,
        "peer".into(),
        uuid::Uuid::from_u128(1),
    );

    assert!(receiver.try_recv().is_err());
}

#[test]
fn same_user_video_sessions_remain_independent_when_one_leaves() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    let second_track_id = uuid::Uuid::from_u128(2);
    let mut second_frame = frame(0, true);
    second_frame.track_id = second_track_id;
    let second_session = cheenhub_contracts::realtime::VoiceRoomParticipant {
        session_id: second_track_id.to_string(),
        ..participant_session("peer", "2026-10-28T09:00:00Z")
    };

    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.publish_frame(ParticipantVideoSource::ScreenShare, second_frame.clone());
        handle.reconcile_participants(
            &[
                participant_session("peer", "2026-10-28T09:00:00Z"),
                second_session.clone(),
            ],
            &[second_session],
        );
    });

    assert_eq!(
        handle.live_tracks(ParticipantVideoSource::ScreenShare),
        vec![("peer".to_owned(), second_track_id)],
    );
    let mut remaining_frames = handle.subscribe_stream(
        ParticipantVideoSource::ScreenShare,
        "peer".to_owned(),
        second_track_id,
    );
    assert_eq!(remaining_frames.try_recv().unwrap(), second_frame);
}

#[test]
fn stream_expiry_updates_state_without_hoisting_the_provider_signal() {
    crate::tests::native::with_runtime(async {
        let warnings = Arc::new(Mutex::new(Vec::new()));
        let _subscriber = tracing::subscriber::set_default(SignalWarnings(warnings.clone()));
        let mut dom = VirtualDom::new(scenario_app);
        let mut scenario = dom.in_scope(ScopeId::ROOT, || VideoOwnerScenario {
            visible: Signal::new(true),
            generations: Rc::new(RefCell::new(Weak::new())),
        });
        dom.provide_root_context(scenario.clone());
        dom.rebuild_in_place();
        let generations = scenario.generations.borrow().upgrade().unwrap();
        dom.render_immediate_to_vec();
        crate::tests::native::wait_until(&mut dom, || generations.borrow().is_empty()).await;
        assert!(
            generations.borrow().is_empty(),
            "expiry task must clear the inactive stream"
        );

        dom.in_scope(ScopeId::ROOT, || scenario.visible.set(false));
        dom.render_immediate_to_vec();

        assert!(
            warnings.lock().unwrap().is_empty(),
            "expiry must not access the provider signal from the root scope"
        );
    });
}

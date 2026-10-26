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
            Rc::new(RefCell::new(HashSet::new())),
            dioxus::core::current_scope_id(),
        );
        handle.mark_live(ParticipantVideoKey::new(
            ParticipantVideoSource::Camera,
            "peer".to_owned(),
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
            Rc::new(RefCell::new(HashSet::new())),
            ScopeId::ROOT,
        )
    })
}

fn frame(sequence: u64, key_frame: bool) -> ParticipantVideoFrame {
    ParticipantVideoFrame {
        room_id: "room".into(),
        sender_user_id: "peer".into(),
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

    let mut frames = handle.subscribe_stream(ParticipantVideoSource::ScreenShare, "peer".into());

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
            handle.live_user_ids(ParticipantVideoSource::ScreenShare),
            vec!["peer"]
        );
        handle.release_stream(ParticipantVideoSource::ScreenShare, "peer");
        assert!(
            handle
                .live_user_ids(ParticipantVideoSource::ScreenShare)
                .is_empty()
        );
    });
}

#[test]
fn explicit_stop_discards_buffered_frames_and_restart_waits_for_a_fresh_key_frame() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.release_stream(ParticipantVideoSource::ScreenShare, "peer");
    });
    let mut receiver = handle.subscribe_stream(ParticipantVideoSource::ScreenShare, "peer".into());
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
fn leaving_the_call_discards_buffered_screen_frames() {
    let dom = VirtualDom::new(|| rsx! {});
    let handle = video_handle(&dom);
    dom.in_scope(ScopeId::ROOT, || {
        handle.publish_frame(ParticipantVideoSource::ScreenShare, frame(0, true));
        handle.clear();
    });

    let mut receiver = handle.subscribe_stream(ParticipantVideoSource::ScreenShare, "peer".into());

    assert!(receiver.try_recv().is_err());
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

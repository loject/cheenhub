//! Регрессии времени жизни отправки демонстрации экрана при смене вкладки.

use super::*;
use crate::features::realtime::create_handle;
use crate::features::screen_share::ScreenShareCodec;
use futures_util::FutureExt;

#[derive(Clone)]
struct SenderScenario {
    pending: Rc<RefCell<PendingScreenFrames>>,
    wake_receiver: Rc<RefCell<Option<mpsc::Receiver<()>>>>,
    controls_visible: Signal<bool>,
}

fn scenario_app() -> Element {
    let scenario = use_context::<SenderScenario>();
    if (scenario.controls_visible)() {
        rsx! { SenderControls {} }
    } else {
        rsx! { p { "Сообщения" } }
    }
}

#[component]
fn SenderControls() -> Element {
    let scenario = use_context::<SenderScenario>();
    use_hook(move || {
        spawn_screen_frame_sender(
            scenario.controls_visible.origin_scope(),
            scenario.wake_receiver.borrow_mut().take().unwrap(),
            scenario.pending,
            create_handle(),
            "server".into(),
            "00000000-0000-0000-0000-000000000001".into(),
        );
    });
    rsx! { button { "Демонстрация экрана" } }
}

fn frame(sequence: u64) -> EncodedScreenShareFrame {
    EncodedScreenShareFrame {
        sequence,
        timestamp_us: sequence * 33_333,
        duration_us: 33_333,
        codec: ScreenShareCodec::Vp9,
        key_frame: true,
        width: 1280,
        height: 720,
        bytes: vec![1, 2, 3],
    }
}

fn pump(dom: &mut VirtualDom) {
    for _ in 0..5 {
        let _ = dom.wait_for_work().now_or_never();
        dom.render_immediate_to_vec();
    }
}

#[test]
fn screen_sender_consumes_frames_after_controls_are_unmounted() {
    let mut dom = VirtualDom::new(scenario_app);
    let (mut wake_sender, wake_receiver) = mpsc::channel(1);
    let pending = Rc::new(RefCell::new(PendingScreenFrames::default()));
    let scenario = dom.in_scope(ScopeId::ROOT, || SenderScenario {
        pending: pending.clone(),
        wake_receiver: Rc::new(RefCell::new(Some(wake_receiver))),
        controls_visible: Signal::new(true),
    });
    dom.provide_root_context(scenario.clone());
    dom.rebuild_in_place();
    pending.borrow_mut().push(frame(1));
    wake_sender.try_send(()).unwrap();
    pump(&mut dom);
    assert!(pending.borrow_mut().take().is_none());

    let mut visible = scenario.controls_visible;
    dom.in_scope(ScopeId::ROOT, || visible.set(false));
    pump(&mut dom);
    pending.borrow_mut().push(frame(2));
    let wake_result = wake_sender.try_send(());
    pump(&mut dom);

    assert!(
        wake_result.is_ok(),
        "sender must survive navigation to messages"
    );
    assert!(
        pending.borrow_mut().take().is_none(),
        "frames must still be consumed"
    );

    drop(dom);
    assert!(
        wake_sender.try_send(()).is_err(),
        "sender must stop with its owner"
    );
}

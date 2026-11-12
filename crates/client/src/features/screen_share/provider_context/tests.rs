use super::register_pending_start_cleanup;
use crate::features::screen_share::backend::ScreenShareStartControl;
use dioxus::core::{Runtime, ScopeId};
use dioxus::prelude::*;
use futures_channel::oneshot;
use futures_util::FutureExt;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static START_CONTROL: RefCell<Option<ScreenShareStartControl>> = const { RefCell::new(None) };
    static START_CANCELLED: RefCell<Option<oneshot::Receiver<()>>> = const { RefCell::new(None) };
    static START_VISIBLE: RefCell<Option<Signal<bool>>> = const { RefCell::new(None) };
    static START_FINISHED: RefCell<Option<Rc<std::cell::Cell<bool>>>> = const { RefCell::new(None) };
}

fn cleanup_owner_app() -> Element {
    let control = START_CONTROL.with(|slot| slot.borrow().clone().expect("test control"));
    let start_control = use_signal(|| Some(control));
    register_pending_start_cleanup(start_control);
    rsx! {}
}

#[component]
fn CaptureProviderScope(start_control: Signal<Option<ScreenShareStartControl>>) -> Element {
    register_pending_start_cleanup(start_control);
    rsx! {}
}

fn root_with_pending_start() -> Element {
    let control = START_CONTROL.with(|slot| slot.borrow().clone().expect("test control"));
    let visible = use_signal(|| true);
    START_VISIBLE.with(|slot| slot.replace(Some(visible)));
    let start_control = use_signal(|| Some(control));
    use_hook(|| {
        let cancelled = START_CANCELLED.with(|slot| {
            slot.borrow_mut()
                .take()
                .expect("test cancellation receiver")
        });
        let finished =
            START_FINISHED.with(|slot| slot.borrow().clone().expect("test completion flag"));
        Runtime::current().in_scope(ScopeId::ROOT, || {
            spawn(async move {
                let _ = cancelled.await;
                finished.set(true);
            });
        });
    });

    rsx! {
        if visible() {
            CaptureProviderScope { start_control }
        }
    }
}

#[test]
fn dropping_provider_scope_cancels_its_pending_start() {
    let control = ScreenShareStartControl::default();
    START_CONTROL.with(|slot| slot.replace(Some(control.clone())));
    let mut dom = VirtualDom::new(cleanup_owner_app);
    dom.rebuild_in_place();

    drop(dom);
    START_CONTROL.with(|slot| slot.replace(None));

    assert!(control.is_cancelled());
}

#[test]
fn root_start_task_observes_cancellation_after_provider_scope_unmounts() {
    let control = ScreenShareStartControl::default();
    let (cancel_sender, cancel_receiver) = oneshot::channel();
    let cancel_sender = RefCell::new(Some(cancel_sender));
    control.register_cleanup(Rc::new(move || {
        if let Some(sender) = cancel_sender.borrow_mut().take() {
            let _ = sender.send(());
        }
    }));
    let finished = Rc::new(std::cell::Cell::new(false));
    START_CONTROL.with(|slot| slot.replace(Some(control.clone())));
    START_CANCELLED.with(|slot| slot.replace(Some(cancel_receiver)));
    START_FINISHED.with(|slot| slot.replace(Some(finished.clone())));
    let mut dom = VirtualDom::new(root_with_pending_start);
    dom.rebuild_in_place();

    let mut visible = START_VISIBLE.with(|slot| slot.borrow().expect("visible signal"));
    dom.in_scope(ScopeId::ROOT, || visible.set(false));
    dom.rebuild_in_place();
    for _ in 0..5 {
        let _ = dom.wait_for_work().now_or_never();
        dom.render_immediate_to_vec();
    }

    assert!(control.is_cancelled());
    assert!(finished.get());
    START_CONTROL.with(|slot| slot.replace(None));
    START_VISIBLE.with(|slot| slot.replace(None));
    START_FINISHED.with(|slot| slot.replace(None));
    drop(dom);
}

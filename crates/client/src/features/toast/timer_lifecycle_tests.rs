//! Регрессионная проверка жизненного цикла toast-таймера.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use dioxus::dioxus_core::{ScopeId, VirtualDom};
use dioxus::prelude::*;
use futures_channel::oneshot;

static COUNTDOWN_OWNER_DROPPED: AtomicBool = AtomicBool::new(false);

struct CountdownOwnerDropGuard;

impl Drop for CountdownOwnerDropGuard {
    fn drop(&mut self) {
        COUNTDOWN_OWNER_DROPPED.store(true, Ordering::SeqCst);
    }
}

thread_local! {
    static COUNTDOWN_RECEIVER: RefCell<Option<oneshot::Receiver<()>>> = const { RefCell::new(None) };
    static COUNTDOWN_OWNER_VISIBLE: RefCell<Option<Signal<bool>>> = const { RefCell::new(None) };
}

fn countdown_lifecycle_test_app() -> Element {
    let visible = use_signal(|| true);
    COUNTDOWN_OWNER_VISIBLE.with_borrow_mut(|slot| *slot = Some(visible));

    if visible() {
        rsx! { CountdownOwner {} }
    } else {
        rsx! {}
    }
}

#[component]
fn CountdownOwner() -> Element {
    use_hook(|| Rc::new(CountdownOwnerDropGuard));
    use_hook(|| {
        let receiver = COUNTDOWN_RECEIVER
            .with_borrow_mut(Option::take)
            .expect("тест передаёт receiver до монтирования");
        super::timer::spawn_scheduler_task(async move {
            let _ = receiver.await;
        });
    });

    rsx! {}
}

#[test]
fn scheduler_task_stops_with_owning_provider() {
    COUNTDOWN_OWNER_DROPPED.store(false, Ordering::SeqCst);
    COUNTDOWN_OWNER_VISIBLE.with_borrow_mut(Option::take);
    let (sender, receiver) = oneshot::channel();
    COUNTDOWN_RECEIVER.with_borrow_mut(|slot| *slot = Some(receiver));

    let mut dom = VirtualDom::new(countdown_lifecycle_test_app);
    dom.rebuild_in_place();

    dom.in_scope(ScopeId::ROOT, || {
        let mut visible = COUNTDOWN_OWNER_VISIBLE
            .with_borrow(Option::clone)
            .expect("тест сохраняет signal видимости provider");
        visible.set(false);
    });
    dom.render_immediate_to_vec();

    assert!(
        COUNTDOWN_OWNER_DROPPED.load(Ordering::SeqCst),
        "тест должен сначала размонтировать provider"
    );
    assert!(
        sender.send(()).is_err(),
        "scheduler не должен жить дольше владеющего им provider"
    );
}

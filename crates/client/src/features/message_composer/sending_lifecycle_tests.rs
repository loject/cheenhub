//! Регрессионные проверки отмены отправки при смене представления чата.

use super::{MessageComposeState, MessageOperations, use_message_compose_state};
use dioxus::prelude::*;
use futures_util::FutureExt;
use std::cell::RefCell;

thread_local! {
    static STATE: RefCell<Option<MessageComposeState>> = const { RefCell::new(None) };
    static VISIBLE: RefCell<Option<Signal<bool>>> = const { RefCell::new(None) };
}

fn lifecycle_app() -> Element {
    let state = use_message_compose_state();
    let visible = use_signal(|| true);
    STATE.with_borrow_mut(|slot| *slot = Some(state));
    VISIBLE.with_borrow_mut(|slot| *slot = Some(visible));
    if visible() {
        rsx! { SendingOwner { state } }
    } else {
        rsx! {}
    }
}

#[component]
fn SendingOwner(state: MessageComposeState) -> Element {
    let operations = MessageOperations {
        upload: use_callback(move |_| async { Ok("image".to_owned()) }.boxed_local()),
        send: use_callback(move |_| futures_util::future::pending().boxed_local()),
    };
    use_hook(move || {
        let mut state = state;
        state.draft.set("черновик".to_owned());
        super::sending::submit_message(state, operations, EventHandler::new(|_| {}));
    });
    rsx! {}
}

#[test]
fn canceled_send_unlocks_persistent_draft() {
    let mut dom = VirtualDom::new(lifecycle_app);
    dom.rebuild_in_place();
    dom.in_scope(ScopeId::ROOT, || {
        let state = STATE.with_borrow(|slot| slot.unwrap());
        assert!((state.is_sending)());
        let mut visible = VISIBLE.with_borrow(|slot| slot.unwrap());
        visible.set(false);
    });
    dom.render_immediate_to_vec();
    dom.in_scope(ScopeId::ROOT, || {
        let state = STATE.with_borrow(|slot| slot.unwrap());
        assert!(
            !(state.is_sending)(),
            "отмена не должна блокировать сохранённую форму"
        );
        assert_eq!((state.draft)(), "черновик");
    });
}

#[test]
fn retry_reuses_uploaded_image_and_preserves_draft_until_success() {
    use super::pending_attachment::pending_image_attachment;
    use std::{cell::Cell, rc::Rc};
    let uploads = Rc::new(Cell::new(0));
    let sends = Rc::new(Cell::new(0));
    let mut dom = VirtualDom::new(|| rsx! {});
    let (state, operations) = dom.in_scope(ScopeId::ROOT, || {
        let state = MessageComposeState {
            draft: Signal::new("  сообщение  ".to_owned()),
            status: Signal::new(String::new()),
            is_sending: Signal::new(false),
            is_selecting_image: Signal::new(false),
            is_reading_clipboard: Signal::new(false),
            pending_attachment: Signal::new(Some(
                pending_image_attachment(None, b"GIF89a".to_vec(), 1024).unwrap(),
            )),
        };
        let upload_count = uploads.clone();
        let send_count = sends.clone();
        let operations = MessageOperations {
            upload: Callback::new(move |_| {
                upload_count.set(upload_count.get() + 1);
                async { Ok("image-id".to_owned()) }.boxed_local()
            }),
            send: Callback::new(move |(body, image_id): (String, Option<String>)| {
                assert_eq!(body, "сообщение");
                assert_eq!(image_id.as_deref(), Some("image-id"));
                send_count.set(send_count.get() + 1);
                let fail = send_count.get() == 1;
                async move {
                    if fail {
                        Err("ошибка отправки".to_owned())
                    } else {
                        Ok(())
                    }
                }
                .boxed_local()
            }),
        };
        super::sending::submit_message(state, operations, EventHandler::new(|_| {}));
        (state, operations)
    });
    dom.render_immediate_to_vec();
    dom.in_scope(ScopeId::ROOT, || {
        assert_eq!((state.draft)(), "  сообщение  ");
        assert_eq!((state.status)(), "ошибка отправки");
        assert_eq!(
            (state.pending_attachment)().unwrap().uploaded_id.as_deref(),
            Some("image-id")
        );
        assert!(!(state.is_sending)());
        super::sending::submit_message(state, operations, EventHandler::new(|_| {}));
    });
    dom.render_immediate_to_vec();
    dom.in_scope(ScopeId::ROOT, || {
        assert!((state.draft)().is_empty());
        assert!((state.pending_attachment)().is_none());
        assert!((state.status)().is_empty());
        assert!(!(state.is_sending)());
    });
    assert_eq!(uploads.get(), 1);
    assert_eq!(sends.get(), 2);
}

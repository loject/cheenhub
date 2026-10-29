//! Регрессионные проверки отмены отправки при смене представления чата.

use super::super::{MessageComposeState, MessageOperations, use_message_compose_state};
use crate::features::typing::TypingNotifier;
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
        max_body_chars: 2000,
        upload: use_callback(move |_| async { Ok("image".to_owned()) }.boxed_local()),
        send: use_callback(move |_| futures_util::future::pending().boxed_local()),
        typing: TypingNotifier::new_for_tests(),
    };
    use_hook(move || {
        let mut state = state;
        state.draft.set("черновик".to_owned());
        super::super::sending::submit_message(state, operations, EventHandler::new(|_| {}));
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
    use super::super::pending_attachment::pending_image_attachment;
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
            max_body_chars: 2000,
            upload: Callback::new(move |_| {
                upload_count.set(upload_count.get() + 1);
                async { Ok("image-id".to_owned()) }.boxed_local()
            }),
            typing: TypingNotifier::new_for_tests(),
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
        super::super::sending::submit_message(state, operations, EventHandler::new(|_| {}));
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
        super::super::sending::submit_message(state, operations, EventHandler::new(|_| {}));
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

#[test]
fn long_text_prefers_paragraph_boundary() {
    let (first, rest) =
        super::super::sending::split_first_part("first paragraph\n\nsecond paragraph", 22);
    assert_eq!(first.trim(), "first paragraph");
    assert_eq!(rest.trim_start(), "second paragraph");
}

#[test]
fn long_text_prefers_whole_words() {
    let (first, rest) = super::super::sending::split_first_part("hello world again", 8);
    assert_eq!(first.trim(), "hello");
    assert_eq!(rest.trim_start(), "world again");
}

#[test]
fn long_word_splits_on_unicode_character_boundary() {
    let (first, rest) = super::super::sending::split_first_part("🙂🙂🙂🙂", 2);
    assert_eq!(first, "🙂🙂");
    assert_eq!(rest, "🙂🙂");
    assert_eq!(first.chars().count(), 2);
}

#[test]
fn text_at_limit_remains_one_message() {
    let (first, rest) = super::super::sending::split_first_part("абвг", 4);
    assert_eq!(first, "абвг");
    assert!(rest.is_empty());
}

#[test]
fn failure_after_first_part_preserves_only_unsent_text_on_retry() {
    use super::super::pending_attachment::pending_image_attachment;
    use std::rc::Rc;

    let uploads = Rc::new(std::cell::Cell::new(0));
    let sent = Rc::new(RefCell::new(Vec::<(String, Option<String>)>::new()));
    let mut dom = VirtualDom::new(|| rsx! {});
    let (state, operations) = dom.in_scope(ScopeId::ROOT, || {
        let state = MessageComposeState {
            draft: Signal::new("hello world again".to_owned()),
            status: Signal::new(String::new()),
            is_sending: Signal::new(false),
            is_selecting_image: Signal::new(false),
            is_reading_clipboard: Signal::new(false),
            pending_attachment: Signal::new(Some(
                pending_image_attachment(None, b"GIF89a".to_vec(), 1024).unwrap(),
            )),
        };
        let uploads_for_callback = uploads.clone();
        let sent_for_callback = sent.clone();
        let operations = MessageOperations {
            max_body_chars: 6,
            upload: Callback::new(move |_| {
                uploads_for_callback.set(uploads_for_callback.get() + 1);
                async { Ok("image-id".to_owned()) }.boxed_local()
            }),
            send: Callback::new(move |value: (String, Option<String>)| {
                sent_for_callback.borrow_mut().push(value);
                let should_fail = sent_for_callback.borrow().len() == 2;
                async move {
                    if should_fail {
                        Err("временная ошибка".to_owned())
                    } else {
                        Ok(())
                    }
                }
                .boxed_local()
            }),
            typing: TypingNotifier::new_for_tests(),
        };
        super::super::sending::submit_message(state, operations, EventHandler::new(|_| {}));
        (state, operations)
    });
    dom.render_immediate_to_vec();
    dom.in_scope(ScopeId::ROOT, || {
        assert_eq!((state.draft)(), "world again");
        assert_eq!((state.status)(), "временная ошибка");
        assert!((state.pending_attachment)().is_none());
        assert!(!(state.is_sending)());
        super::super::sending::submit_message(state, operations, EventHandler::new(|_| {}));
    });
    dom.render_immediate_to_vec();
    dom.in_scope(ScopeId::ROOT, || {
        assert!((state.draft)().is_empty());
        assert!((state.status)().is_empty());
        assert!((state.pending_attachment)().is_none());
        assert!(!(state.is_sending)());
    });
    assert_eq!(uploads.get(), 1);
    assert_eq!(
        *sent.borrow(),
        vec![
            ("hello".to_owned(), Some("image-id".to_owned())),
            ("world".to_owned(), None),
            ("world".to_owned(), None),
            ("again".to_owned(), None),
        ]
    );
}

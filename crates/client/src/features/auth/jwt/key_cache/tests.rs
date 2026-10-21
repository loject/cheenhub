use std::{cell::Cell, future::Future, rc::Rc};

use cheenhub_contracts::rest::AuthVerificationKey;
use futures_channel::oneshot;
use futures_util::FutureExt;

use super::super::JwtVerifyError;
use super::VerificationKeyCache;

type ResultKey = Result<AuthVerificationKey, JwtVerifyError>;

fn key() -> AuthVerificationKey {
    AuthVerificationKey {
        key_id: "runtime-key".to_owned(),
        public_key_base64: "FyeAHCHdj3LQJcxcJv1Zo3mW8m+kqBGytTetC2NCIBU=".to_owned(),
    }
}

fn request(
    calls: Rc<Cell<usize>>,
) -> (
    oneshot::Sender<ResultKey>,
    impl Future<Output = ResultKey> + 'static,
) {
    let (sender, receiver) = oneshot::channel();
    let future = async move {
        calls.set(calls.get() + 1);
        receiver
            .await
            .expect("управляемый запрос должен завершиться")
    };
    (sender, future)
}

#[test]
fn concurrent_consumers_share_one_fetch_and_reuse_the_successful_key() {
    let cache = VerificationKeyCache::default();
    let calls = Rc::new(Cell::new(0));
    let (sender, fetch) = request(calls.clone());
    let (_unused_sender, unused_fetch) = request(calls.clone());
    let mut first = Box::pin(cache.load(fetch));
    let mut second = Box::pin(cache.load(unused_fetch));

    assert!(first.as_mut().now_or_never().is_none());
    assert!(second.as_mut().now_or_never().is_none());
    assert_eq!(calls.get(), 1);
    sender.send(Ok(key())).unwrap();

    assert_eq!(first.as_mut().now_or_never(), Some(Ok(key())));
    assert_eq!(second.as_mut().now_or_never(), Some(Ok(key())));
    let (_sender, fetch) = request(calls.clone());
    assert_eq!(cache.load(fetch).now_or_never(), Some(Ok(key())));
    assert_eq!(calls.get(), 1);
}

#[test]
fn failed_fetch_is_shared_but_does_not_prevent_a_later_retry() {
    let cache = VerificationKeyCache::default();
    let calls = Rc::new(Cell::new(0));
    let (sender, fetch) = request(calls.clone());
    let (_unused_sender, unused_fetch) = request(calls.clone());
    let mut first = Box::pin(cache.load(fetch));
    let mut second = Box::pin(cache.load(unused_fetch));
    assert!(first.as_mut().now_or_never().is_none());
    assert!(second.as_mut().now_or_never().is_none());

    sender
        .send(Err(JwtVerifyError::VerificationKeyUnavailable))
        .unwrap();
    assert_eq!(
        first.as_mut().now_or_never(),
        Some(Err(JwtVerifyError::VerificationKeyUnavailable))
    );
    assert_eq!(
        second.as_mut().now_or_never(),
        Some(Err(JwtVerifyError::VerificationKeyUnavailable))
    );
    assert_eq!(calls.get(), 1);

    let (sender, fetch) = request(calls.clone());
    let mut retry = Box::pin(cache.load(fetch));
    assert!(retry.as_mut().now_or_never().is_none());
    sender.send(Ok(key())).unwrap();
    assert_eq!(retry.as_mut().now_or_never(), Some(Ok(key())));
    assert_eq!(calls.get(), 2);
}

#[test]
fn cancelling_the_first_consumer_keeps_the_shared_fetch_available() {
    let cache = VerificationKeyCache::default();
    let calls = Rc::new(Cell::new(0));
    let (sender, fetch) = request(calls.clone());
    let mut first = Box::pin(cache.load(fetch));
    assert!(first.as_mut().now_or_never().is_none());

    drop(first);
    sender.send(Ok(key())).unwrap();

    let (_sender, fetch) = request(calls.clone());
    assert_eq!(cache.load(fetch).now_or_never(), Some(Ok(key())));
    assert_eq!(calls.get(), 1);
}

#[test]
fn an_old_failed_consumer_does_not_clear_a_new_pending_retry() {
    let cache = VerificationKeyCache::default();
    let calls = Rc::new(Cell::new(0));
    let (sender, fetch) = request(calls.clone());
    let (_unused_sender, unused_fetch) = request(calls.clone());
    let mut first = Box::pin(cache.load(fetch));
    let mut second = Box::pin(cache.load(unused_fetch));
    assert!(first.as_mut().now_or_never().is_none());
    assert!(second.as_mut().now_or_never().is_none());
    sender
        .send(Err(JwtVerifyError::VerificationKeyUnavailable))
        .unwrap();
    assert!(matches!(first.as_mut().now_or_never(), Some(Err(_))));

    let (sender, fetch) = request(calls.clone());
    let mut retry = Box::pin(cache.load(fetch));
    assert!(retry.as_mut().now_or_never().is_none());
    assert!(matches!(second.as_mut().now_or_never(), Some(Err(_))));
    let (_last_sender, last_fetch) = request(calls.clone());
    let mut third = Box::pin(cache.load(last_fetch));
    assert!(third.as_mut().now_or_never().is_none());
    assert_eq!(calls.get(), 2);

    sender.send(Ok(key())).unwrap();
    assert_eq!(retry.as_mut().now_or_never(), Some(Ok(key())));
    assert_eq!(third.as_mut().now_or_never(), Some(Ok(key())));
}

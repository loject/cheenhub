use super::*;
use futures_util::FutureExt;

#[test]
fn late_worker_ack_does_not_complete_new_denoise_request() {
    let control = DenoiseControl::default();
    let (sender, mut receiver) = oneshot::channel();
    *control.0.borrow_mut() = Some((9, sender));

    control.complete(5, Ok(()));

    assert!((&mut receiver).now_or_never().is_none());
    control.complete(9, Ok(()));
    assert!(receiver.now_or_never().unwrap().unwrap().is_ok());
}

#[test]
fn worker_rejection_reaches_the_matching_transaction() {
    let control = DenoiseControl::default();
    let (sender, receiver) = oneshot::channel();
    *control.0.borrow_mut() = Some((9, sender));

    control.complete(9, Err(MicrophoneError::new("processor rejected mode")));

    assert!(receiver.now_or_never().unwrap().unwrap().is_err());
}

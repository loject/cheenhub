use super::*;
use futures_util::{
    FutureExt,
    future::{pending, ready},
};

#[test]
fn unresolved_constraints_can_be_cancelled_without_waiting_for_them() {
    let (sender, cancelled) = oneshot::channel();
    sender.send(()).unwrap();

    let outcome = wait(pending(), cancelled, pending())
        .now_or_never()
        .unwrap();

    assert!(matches!(outcome, Outcome::Cancelled));
}

#[test]
fn unresolved_constraints_time_out() {
    let (_sender, cancelled) = oneshot::channel();

    let outcome = wait(pending(), cancelled, ready(()))
        .now_or_never()
        .unwrap();

    assert!(matches!(outcome, Outcome::TimedOut));
}

#[test]
fn constraints_rejection_is_reported_without_committing_the_request() {
    let (_sender, cancelled) = oneshot::channel();

    let outcome = wait(
        ready(Err(MicrophoneError::new("constraints rejected"))),
        cancelled,
        pending(),
    )
    .now_or_never()
    .unwrap();

    assert!(matches!(outcome, Outcome::Applied(Err(_))));
}

mod platform;

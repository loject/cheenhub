use super::*;
use futures_util::{FutureExt, future::ready};

#[test]
fn rejected_constraints_restore_previous_browser_mode() {
    let calls = Rc::new(std::cell::RefCell::new(Vec::new()));
    let record = calls.clone();

    let result = constraints(DenoiseMode::Rnnoise, DenoiseMode::Browser, move |mode| {
        record.borrow_mut().push(mode);
        ready(if mode == DenoiseMode::Rnnoise {
            Err(MicrophoneError::new("rejected"))
        } else {
            Ok(())
        })
    })
    .now_or_never()
    .unwrap();

    assert!(result.is_err());
    assert!(!result.unwrap_err().is_invalidated());
    assert_eq!(
        *calls.borrow(),
        [DenoiseMode::Rnnoise, DenoiseMode::Browser]
    );
}

#[test]
fn failed_constraints_rollback_invalidates_session() {
    let result = constraints(DenoiseMode::Off, DenoiseMode::Browser, |_| {
        ready(Err(MicrophoneError::new("rejected")))
    })
    .now_or_never()
    .unwrap();

    assert!(result.unwrap_err().is_invalidated());
}

#[test]
fn constraints_only_change_when_crossing_browser_boundary() {
    let modes = [DenoiseMode::Off, DenoiseMode::Browser, DenoiseMode::Rnnoise];
    for previous in modes {
        for next in modes {
            let calls = Rc::new(Cell::new(0));
            let record = calls.clone();
            let result = constraints(next, previous, move |_| {
                record.set(record.get() + 1);
                ready(Ok(()))
            })
            .now_or_never()
            .unwrap();

            assert!(result.is_ok());
            assert_eq!(
                calls.get(),
                usize::from((next == DenoiseMode::Browser) != (previous == DenoiseMode::Browser)),
                "{previous:?} -> {next:?}"
            );
        }
    }
}

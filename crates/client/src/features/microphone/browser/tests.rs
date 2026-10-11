use super::*;

#[test]
fn encoder_close_failure_still_closes_audio_context() {
    let closed = Cell::new(false);
    let result = finish_cleanup(Err(MicrophoneError::new("encoder already closed")), async {
        closed.set(true);
        Ok(())
    })
    .now_or_never()
    .unwrap();

    assert!(closed.get());
    assert!(result.is_err());
}

#[test]
fn audio_context_close_failure_is_reported() {
    let result = finish_cleanup(Ok(()), async {
        Err(MicrophoneError::new("context close rejected"))
    })
    .now_or_never()
    .unwrap();

    assert!(result.is_err());
}

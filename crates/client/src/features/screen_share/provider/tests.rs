use super::deliver_start_result;
use crate::features::screen_share::backend::{ScreenShareSession, ScreenShareStartControl};
use futures_channel::oneshot;
use futures_util::FutureExt;
use std::cell::Cell;
use std::rc::Rc;

struct TestSession(Rc<Cell<usize>>);

impl ScreenShareSession for TestSession {
    fn take_audio_frames(
        &self,
    ) -> Option<
        futures_channel::mpsc::Receiver<
            crate::features::screen_share::backend::EncodedScreenAudioFrame,
        >,
    > {
        None
    }

    fn stop(
        &self,
    ) -> futures_util::future::LocalBoxFuture<
        'static,
        Result<(), crate::features::screen_share::backend::ScreenShareError>,
    > {
        let stop_count = self.0.clone();
        Box::pin(async move {
            stop_count.set(stop_count.get() + 1);
            Ok(())
        })
    }
}

#[test]
fn late_start_result_is_stopped_when_provider_receiver_was_dropped() {
    let stop_count = Rc::new(Cell::new(0));
    let session: Rc<dyn ScreenShareSession> = Rc::new(TestSession(stop_count.clone()));
    let control = ScreenShareStartControl::default();
    let (sender, receiver) = oneshot::channel();
    drop(receiver);

    deliver_start_result(Ok(session), control, sender)
        .now_or_never()
        .expect("result delivery and cleanup should finish immediately");

    assert_eq!(stop_count.get(), 1);
}

#[test]
fn cancelled_start_result_is_stopped_without_reaching_provider() {
    let stop_count = Rc::new(Cell::new(0));
    let session: Rc<dyn ScreenShareSession> = Rc::new(TestSession(stop_count.clone()));
    let control = ScreenShareStartControl::default();
    control.cancel();
    let (sender, receiver) = oneshot::channel();

    deliver_start_result(Ok(session), control, sender)
        .now_or_never()
        .expect("cancelled result cleanup should finish immediately");

    assert_eq!(stop_count.get(), 1);
    assert!(
        receiver
            .now_or_never()
            .expect("sender was dropped")
            .is_err()
    );
}

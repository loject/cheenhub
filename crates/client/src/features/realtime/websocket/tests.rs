use super::*;
use futures_util::{FutureExt, future::Abortable};
use std::{cell::Cell, rc::Rc};

#[test]
fn shutdown_interrupts_blocked_writer_and_reader_without_draining_queue() {
    struct Dropped(Rc<Cell<bool>>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }
    let (sender, _receiver) = mpsc::unbounded();
    let cloned_sender = sender.clone();
    let (sender, writer_cancel, reader_cancel) = WebSocketOutboundSender::new(sender);
    let writer_dropped = Rc::new(Cell::new(false));
    let reader_dropped = Rc::new(Cell::new(false));
    let writer_guard = Dropped(writer_dropped.clone());
    let reader_guard = Dropped(reader_dropped.clone());
    let mut writer = Box::pin(async {
        Abortable::new(
            async move {
                let _guard = writer_guard;
                std::future::pending::<()>().await;
            },
            writer_cancel,
        )
        .await
    });
    let mut reader = Box::pin(async {
        Abortable::new(
            async move {
                let _guard = reader_guard;
                std::future::pending::<()>().await;
            },
            reader_cancel,
        )
        .await
    });
    assert!(writer.as_mut().now_or_never().is_none());
    assert!(reader.as_mut().now_or_never().is_none());
    sender
        .unbounded_send(WebSocketOutbound::Datagram(bytes::Bytes::from_static(
            b"queued",
        )))
        .unwrap();
    sender.close();
    assert!(matches!(writer.as_mut().now_or_never(), Some(Err(_))));
    assert!(matches!(reader.as_mut().now_or_never(), Some(Err(_))));
    assert!(writer_dropped.get());
    assert!(reader_dropped.get());
    assert!(cloned_sender.is_closed());
}

use super::close_once;
use std::cell::Cell;

#[test]
fn closed_video_frame_is_not_closed_twice() {
    let closed = Cell::new(false);
    let calls = Cell::new(0);

    let first = close_once(&closed, || {
        calls.set(calls.get() + 1);
        Ok::<(), ()>(())
    });
    let second = close_once(&closed, || {
        calls.set(calls.get() + 1);
        Ok::<(), ()>(())
    });

    assert_eq!(first, Ok(true));
    assert_eq!(second, Ok(false));
    assert_eq!(calls.get(), 1);
}

#[test]
fn video_frame_close_error_is_reported_and_not_retried() {
    let closed = Cell::new(false);
    let calls = Cell::new(0);

    let first = close_once(&closed, || {
        calls.set(calls.get() + 1);
        Err("VideoFrame.close failed")
    });
    let second = close_once(&closed, || {
        calls.set(calls.get() + 1);
        Ok::<(), &str>(())
    });

    assert_eq!(first, Err("VideoFrame.close failed"));
    assert_eq!(second, Ok(false));
    assert!(closed.get());
    assert_eq!(calls.get(), 1);
}

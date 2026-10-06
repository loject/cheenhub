use super::*;

#[test]
fn pcm_decoding_preserves_native_float_samples() {
    let bytes: Vec<u8> = [0.25_f32, -0.5, 2.0]
        .into_iter()
        .flat_map(f32::to_ne_bytes)
        .collect();
    assert_eq!(decode_samples(&bytes), vec![0.25, -0.5, 1.0]);
}

#[test]
fn startup_deadline_is_bounded_without_a_server() {
    let mut mainloop = Mainloop::new().expect("mainloop");
    assert!(tick(&mut mainloop, Some(Instant::now())).is_err());
}

#[test]
fn invalid_source_is_rejected_before_calling_pulse() {
    let (pcm, _receiver) = std::sync::mpsc::sync_channel(1);
    let (started, mut startup) = oneshot::channel();
    assert!(
        run(
            Some("source\0invalid"),
            &AtomicBool::new(false),
            pcm,
            started
        )
        .is_err()
    );
    assert!(
        startup
            .try_recv()
            .expect("startup channel")
            .expect("startup result")
            .is_err()
    );
}

#[test]
#[ignore = "Требуется запущенный звуковой сервер и доступ к микрофону"]
fn live_default_capture_starts_and_stops_without_saving_audio() {
    let closed = std::sync::Arc::new(AtomicBool::new(false));
    let _cancellation = super::super::Cancellation(closed.clone());
    let worker_closed = closed.clone();
    let (pcm, receiver) = std::sync::mpsc::sync_channel(48);
    let (started, mut startup) = oneshot::channel();
    let (finished, completion) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let result = run(None, &worker_closed, pcm, started);
        let _ = finished.send(result);
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(result) = startup.try_recv().expect("startup sender") {
            result.expect("default capture startup");
            break;
        }
        assert!(Instant::now() < deadline, "startup deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
    let captured = receiver.recv_timeout(Duration::from_secs(2));
    closed.store(true, Ordering::Relaxed);
    completion
        .recv_timeout(Duration::from_secs(1))
        .expect("bounded stop")
        .expect("capture stopped");
    worker.join().expect("worker joined");
    let captured = captured.expect("PCM received");
    assert!(!captured.is_empty());
    assert!(captured.iter().all(|sample| sample.is_finite()));
}

#[test]
#[ignore = "Требуется запущенный звуковой сервер"]
fn live_missing_explicit_source_fails_without_default_fallback() {
    let closed = AtomicBool::new(false);
    let (pcm, _receiver) = std::sync::mpsc::sync_channel(48);
    let (started, mut startup) = oneshot::channel();
    assert!(
        run(
            Some("cheenhub-nonexistent-source-7ad7936a"),
            &closed,
            pcm,
            started
        )
        .is_err()
    );
    assert!(
        startup
            .try_recv()
            .expect("startup channel")
            .expect("startup result")
            .is_err()
    );
}

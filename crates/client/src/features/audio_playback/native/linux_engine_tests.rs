//! Проверки жизненного цикла вывода; live-проверки передают только тишину.

use super::*;

#[test]
fn rejects_invalid_device_name_before_starting_worker() {
    assert!(create_engine(Some("sink\0invalid".into()), 1.0, 48_000).is_err());
}

#[test]
#[ignore = "Требуется запущенный PulseAudio или PipeWire Pulse"]
fn live_default_playback_starts_and_stops() {
    let engine = create_engine(None, 0.0, 48_000).expect("output worker");
    let deadline = Instant::now() + Duration::from_secs(4);
    while !engine.ready.load(Ordering::Acquire) {
        assert!(
            !engine.worker.as_ref().unwrap().is_finished(),
            "startup failed"
        );
        assert!(Instant::now() < deadline, "startup timed out");
        thread::sleep(Duration::from_millis(10));
    }
    let stop_started = Instant::now();
    drop(engine);
    assert!(stop_started.elapsed() < Duration::from_secs(1));
}

#[test]
#[ignore = "Требуется запущенный PulseAudio или PipeWire Pulse"]
fn live_missing_output_never_becomes_ready() {
    let ready = AtomicBool::new(false);
    let result = run(
        Some("cheenhub-missing-sink-22d21527"),
        48_000,
        new_mixer(0.0),
        &AtomicBool::new(false),
        &ready,
    );
    assert!(result.is_err());
    assert!(!ready.load(Ordering::Acquire));
}

#[test]
fn dropping_connecting_playback_is_bounded() {
    let engine = create_engine(None, 0.0, 48_000).expect("worker");
    let stop_started = Instant::now();
    drop(engine);
    assert!(stop_started.elapsed() < Duration::from_secs(1));
}

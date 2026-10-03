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
        || false,
        &ready,
        Arc::new(AtomicU64::new(0)),
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

#[test]
fn playback_buffer_reserves_forty_milliseconds_with_server_request_size() {
    for rate in [44_100, 48_000, 96_000] {
        let (attr, flags) = playback_buffer_config(rate, false);
        assert_eq!(attr.tlength, rate * 8 * 40 / 1000);
        assert_eq!(attr.minreq, u32::MAX);
        assert_eq!(attr.prebuf, u32::MAX);
        assert_eq!(attr.maxlength, u32::MAX);
        assert!(!flags.contains(pulse::stream::FlagSet::ADJUST_LATENCY));
        assert!(!flags.contains(pulse::stream::FlagSet::DONT_MOVE));
    }
}

#[test]
fn explicit_playback_device_stays_pinned_without_adjusting_device_latency() {
    let (_, flags) = playback_buffer_config(48_000, true);
    assert_eq!(flags, pulse::stream::FlagSet::DONT_MOVE);
}

#[test]
#[ignore = "Требуется запущенный PulseAudio или PipeWire Pulse и стабильное устройство вывода"]
fn live_silent_playback_runs_without_underruns() {
    let engine = create_engine(None, 0.0, 48_000).expect("output worker");
    let startup_deadline = Instant::now() + Duration::from_secs(4);
    while !engine.ready.load(Ordering::Acquire) {
        assert!(
            !engine.worker.as_ref().unwrap().is_finished(),
            "startup failed"
        );
        assert!(Instant::now() < startup_deadline, "startup timed out");
        thread::sleep(Duration::from_millis(10));
    }
    let deadline = Instant::now() + Duration::from_secs(6);
    while Instant::now() < deadline {
        assert!(
            !engine.worker.as_ref().unwrap().is_finished(),
            "playback stopped"
        );
        thread::sleep(Duration::from_millis(20));
    }
    let underflows = engine.underflows.clone();
    drop(engine);
    assert_eq!(underflows.load(Ordering::Relaxed), 0, "playback underruns");
    // После остановки ни worker, ни callback не должны удерживать счётчик.
    assert_eq!(Arc::strong_count(&underflows), 1);
}

#[test]
#[ignore = "Требуется запущенный PulseAudio или PipeWire Pulse и стабильное устройство вывода"]
fn live_silent_playback_recovers_after_worker_pause() {
    let ready = AtomicBool::new(false);
    let underflows = Arc::new(AtomicU64::new(0));
    let mut started = None::<Instant>;
    let mut paused = false;
    let mut recovered_count = None;
    let mut before_pause_count = 0;
    run(
        None,
        48_000,
        new_mixer(0.0),
        || {
            if !ready.load(Ordering::Acquire) {
                return false;
            }
            let elapsed = started.get_or_insert_with(Instant::now).elapsed();
            if !paused && elapsed >= Duration::from_secs(2) {
                before_pause_count = underflows.load(Ordering::Relaxed);
                thread::sleep(Duration::from_millis(60));
                paused = true;
            }
            if paused && elapsed >= Duration::from_millis(2500) && recovered_count.is_none() {
                let count = underflows.load(Ordering::Relaxed);
                assert!(
                    count > before_pause_count,
                    "pause did not trigger an underrun"
                );
                recovered_count = Some(count);
            }
            elapsed >= Duration::from_secs(5)
        },
        &ready,
        underflows.clone(),
    )
    .expect("continuous playback");
    assert!(paused, "worker pause was not reached");
    assert_eq!(
        underflows.load(Ordering::Relaxed),
        recovered_count.expect("recovery observation was not reached"),
        "unexpected underruns after recovery"
    );
    assert_eq!(Arc::strong_count(&underflows), 1);
}

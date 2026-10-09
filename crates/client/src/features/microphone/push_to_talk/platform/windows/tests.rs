use super::*;

#[test]
fn resolves_left_and_right_modifiers_from_native_event() {
    for (vk, scan, flags, expected) in [
        (0x10, 0x36, 0, 0xa1),
        (0x10, 0x2a, 0, 0xa0),
        (0x11, 0, LLKHF_EXTENDED, 0xa3),
        (0x11, 0, 0, 0xa2),
        (0x12, 0, LLKHF_EXTENDED, 0xa5),
        (0x12, 0, 0, 0xa4),
        (0x70, 0, 0, 0x70),
    ] {
        assert_eq!(key_code(vk, scan, flags), expected);
    }
}

#[test]
fn stalled_event_loop_closes_transmission() {
    let shared = Arc::new(Shared {
        pressed: AtomicBool::new(true),
        epoch: AtomicU64::new(1),
        stopped: AtomicBool::new(false),
        heartbeat_ms: AtomicU64::new(0),
        recording_generation: Arc::new(AtomicU64::new(0)),
        origin: Instant::now() - Duration::from_secs(2),
    });
    let monitor = Monitor {
        shared: shared.clone(),
    };

    assert!(!monitor.pressed());
    shared
        .heartbeat_ms
        .store(elapsed_ms(&shared), Ordering::Release);
    assert!(monitor.pressed());
}

#[test]
fn dropping_monitor_closes_gate_and_requests_worker_shutdown() {
    let shared = Arc::new(Shared {
        pressed: AtomicBool::new(true),
        epoch: AtomicU64::new(1),
        stopped: AtomicBool::new(false),
        heartbeat_ms: AtomicU64::new(0),
        recording_generation: Arc::new(AtomicU64::new(0)),
        origin: Instant::now(),
    });
    let monitor = Monitor {
        shared: shared.clone(),
    };

    drop(monitor);

    assert!(!shared.pressed.load(Ordering::Acquire));
    assert!(shared.stopped.load(Ordering::Acquire));
}

#[test]
fn recording_suspends_capture_and_revokes_previous_hold_on_completion() {
    let generation = Arc::new(AtomicU64::new(0));
    let shared = Arc::new(Shared {
        pressed: AtomicBool::new(true),
        epoch: AtomicU64::new(1),
        stopped: AtomicBool::new(false),
        heartbeat_ms: AtomicU64::new(0),
        origin: Instant::now(),
        recording_generation: generation.clone(),
    });
    let monitor = Monitor { shared };
    let previous = monitor.held_epoch().unwrap();

    generation.fetch_add(1, Ordering::AcqRel);
    assert!(monitor.held_epoch().is_none());
    generation.fetch_add(1, Ordering::AcqRel);

    assert_ne!(monitor.held_epoch().unwrap(), previous);
}

use super::*;

fn active_service(users: usize) -> (Arc<Service>, mpsc::UnboundedReceiver<Command>) {
    let (commands, receiver) = mpsc::unbounded_channel();
    let service = Arc::new(Service {
        state: State::default(),
        available: AtomicBool::new(true),
        users: AtomicUsize::new(users),
        commands,
        stopped: AtomicBool::new(false),
        updates: watch::channel(updates::initial()).0,
    });
    service.state.recover();
    service.state.event(true);
    (service, receiver)
}

#[test]
fn capture_started_during_probe_recovers_without_restarting_microphone() {
    let (service, mut commands) = active_service(0);
    service.state.fail();
    service.available.store(false, Ordering::Release);
    let recording = Arc::new(AtomicU64::new(0));

    let monitor = Monitor::attach(service.clone(), recording).unwrap();

    assert_eq!(service.updates.borrow().availability, Availability::Pending);
    assert_eq!(monitor.held_epoch(), None);
    assert!(matches!(commands.try_recv(), Ok(Command::Ensure)));
    service.available(true);
    service.state.recover();
    service.publish(Status::Ready);
    assert_eq!(monitor.held_epoch(), None);
    service.state.event(true);
    assert!(monitor.held_epoch().is_some());
}

#[test]
fn dropping_last_capture_closes_gate_and_requests_session_release() {
    let (service, mut commands) = active_service(1);
    let recording = Arc::new(AtomicU64::new(0));
    let monitor = Monitor {
        service: service.clone(),
        recording_generation: recording.clone(),
        released: AtomicBool::new(false),
    };
    assert!(monitor.held_epoch().is_some());

    drop(monitor);

    assert_eq!(service.state.held_epoch(&recording), None);
    assert!(matches!(commands.try_recv(), Ok(Command::Release)));
}

#[test]
fn dropping_one_capture_keeps_the_other_capture_lease() {
    let (service, mut commands) = active_service(2);
    let recording = Arc::new(AtomicU64::new(0));
    let monitor = Monitor {
        service: service.clone(),
        recording_generation: recording.clone(),
        released: AtomicBool::new(false),
    };

    drop(monitor);

    assert!(service.state.held_epoch(&recording).is_some());
    assert!(commands.try_recv().is_err());
}

#[test]
fn stopped_service_cannot_reopen_from_late_activation() {
    let (service, _commands) = active_service(1);
    let recording = Arc::new(AtomicU64::new(0));
    let monitor = Monitor {
        service: service.clone(),
        recording_generation: recording,
        released: AtomicBool::new(false),
    };

    service.state.fail();
    service.stopped.store(true, Ordering::Release);
    service.state.recover();
    service.state.event(true);

    assert_eq!(monitor.held_epoch(), None);
}

#[test]
fn worker_unwind_closes_gate_without_waiting_for_watchdog() {
    let (service, _commands) = active_service(1);
    let worker_service = service.clone();
    let recording = AtomicU64::new(0);

    let result = std::thread::spawn(move || {
        let _exit = WorkerExit(worker_service);
        panic!("simulated portal worker panic");
    })
    .join();

    assert!(result.is_err());
    assert_eq!(service.state.held_epoch(&recording), None);
    assert!(service.stopped.load(Ordering::Acquire));
    assert!(!service.available.load(Ordering::Acquire));
    assert!(service.updates.borrow().restart_capture);
}

#[test]
fn worker_exit_preserves_restart_hint_when_capture_start_races() {
    for exit_first in [true, false] {
        let (service, _) = active_service(1);
        service
            .updates
            .send_modify(|update| update.restart_capture = true);

        if exit_first {
            drop(WorkerExit(service.clone()));
            service.capture_started();
        } else {
            service.capture_started();
            assert!(!service.updates.borrow().restart_capture);
            drop(WorkerExit(service.clone()));
        }

        assert!(service.updates.borrow().restart_capture);
        assert_eq!(service.state.held_epoch(&AtomicU64::new(0)), None);
    }
}

#[test]
fn stopping_capture_releases_session_while_queued_frames_hold_monitor() {
    let (service, mut commands) = active_service(1);
    let monitor = Arc::new(Monitor {
        service: service.clone(),
        recording_generation: Arc::new(AtomicU64::new(0)),
        released: AtomicBool::new(false),
    });
    let queued_frame = monitor.clone();

    monitor.stop();

    assert_eq!(queued_frame.held_epoch(), None);
    assert_eq!(service.users.load(Ordering::Acquire), 0);
    assert!(matches!(commands.try_recv(), Ok(Command::Release)));
    drop(monitor);
    drop(queued_frame);
    assert_eq!(service.users.load(Ordering::Acquire), 0);
    assert!(commands.try_recv().is_err());
}

#[test]
fn available_probe_does_not_make_gate_ready() {
    let (service, _) = active_service(1);
    service.available(true);

    service.publish(Status::Available);
    service.state.event(true);

    assert_eq!(
        service.updates.borrow().availability,
        Availability::Available
    );
    assert_eq!(service.updates.borrow().status, Status::Available);
    assert_eq!(service.state.held_epoch(&AtomicU64::new(0)), None);
}

#[test]
fn binding_required_and_failed_states_invalidate_captured_epoch() {
    for status in [
        Status::BindingRequired,
        Status::Failed("Регистрация отклонена".into()),
    ] {
        let (service, _) = active_service(1);
        let recording = AtomicU64::new(0);
        assert!(service.state.held_epoch(&recording).is_some());

        service.publish(status.clone());
        service.state.event(true);

        assert_eq!(service.updates.borrow().status, status);
        assert_eq!(service.state.held_epoch(&recording), None);
    }
}

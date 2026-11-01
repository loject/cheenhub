//! Регрессии пересчёта локального дедлайна по серверному времени.

use std::time::Duration;

use web_time::Instant;

use super::preferred_pending_deadline;

#[test]
fn recovered_server_time_shortens_delayed_pending_event_deadline() {
    let now = Instant::now();
    let late_event_deadline = now + Duration::from_secs(45);
    let server_deadline = now + Duration::from_secs(5);

    assert_eq!(
        preferred_pending_deadline(Some(late_event_deadline), server_deadline, true),
        Some(server_deadline)
    );
}

#[test]
fn stale_recovery_and_duplicate_event_do_not_extend_pending_deadline() {
    let now = Instant::now();
    let earlier_deadline = now + Duration::from_secs(5);
    let later_deadline = now + Duration::from_secs(45);

    assert_eq!(
        preferred_pending_deadline(Some(earlier_deadline), later_deadline, true),
        None
    );
    assert_eq!(
        preferred_pending_deadline(Some(earlier_deadline), later_deadline, false),
        None
    );
    assert_eq!(
        preferred_pending_deadline(None, later_deadline, false),
        Some(later_deadline)
    );
}

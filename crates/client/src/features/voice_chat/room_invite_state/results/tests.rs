//! Регрессии согласования локальных тайм-аутов с окончательными решениями сервера.

use super::*;
use chrono::{DateTime, Utc};

fn time(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

fn invite(
    status: VoiceRoomInviteStatus,
    resolved_at: Option<DateTime<Utc>>,
) -> VoiceRoomInviteSnapshot {
    VoiceRoomInviteSnapshot {
        invite_id: "invite-1".to_owned(),
        server_id: "server-1".to_owned(),
        server_name: "Сервер".to_owned(),
        room_id: "room-1".to_owned(),
        room_name: "Голосовая".to_owned(),
        caller_user_id: "caller".to_owned(),
        caller_nickname: "Отправитель".to_owned(),
        caller_avatar_url: None,
        recipient_user_id: "recipient".to_owned(),
        recipient_nickname: Some("Аня".to_owned()),
        recipient_avatar_url: None,
        status,
        created_at: "2026-10-31T15:00:00Z".to_owned(),
        expires_at: "2026-10-31T15:00:45Z".to_owned(),
        resolved_at: resolved_at.map(|value| value.to_rfc3339()),
    }
}

#[test]
fn authoritative_decline_or_cancellation_replaces_local_unanswered_result() {
    let now = Instant::now();
    for status in [
        VoiceRoomInviteStatus::Declined,
        VoiceRoomInviteStatus::Cancelled,
    ] {
        let mut results = HashMap::new();
        update_recent_result(
            &mut results,
            invite(VoiceRoomInviteStatus::Expired, None),
            now,
        );
        let resolved_at = time("2026-10-31T15:00:43Z");

        let deadline = update_recent_result(
            &mut results,
            invite(status, Some(resolved_at)),
            now + Duration::from_secs(1),
        );

        assert_eq!(deadline, Some(now + Duration::from_secs(11)));
        assert_eq!(results["invite-1"].invite.status, status);
        assert_eq!(
            results["invite-1"].show_until,
            now + Duration::from_secs(11)
        );
    }
}

#[test]
fn recovered_resolution_shows_ten_seconds_from_local_observation() {
    let resolved_at = time("2026-10-31T15:00:20Z");
    let now = Instant::now();
    let mut results = HashMap::new();

    let deadline = update_recent_result(
        &mut results,
        invite(VoiceRoomInviteStatus::Declined, Some(resolved_at)),
        now,
    );
    assert_eq!(deadline, Some(now + Duration::from_secs(10)));
    assert!(results.contains_key("invite-1"));

    let repeated = update_recent_result(
        &mut results,
        invite(VoiceRoomInviteStatus::Declined, Some(resolved_at)),
        now + Duration::from_secs(9),
    );
    assert_eq!(repeated, deadline);

    let expired = update_recent_result(
        &mut results,
        invite(VoiceRoomInviteStatus::Declined, Some(resolved_at)),
        now + Duration::from_secs(10),
    );
    assert!(expired.is_none());
    assert!(results.is_empty());
}

#[test]
fn recovered_resolution_shows_for_local_interval_despite_old_server_timestamp() {
    let mut results = HashMap::new();
    let old_server_resolution = time("2000-01-01T00:00:00Z");
    let local_now = Instant::now();

    let deadline = update_recent_result(
        &mut results,
        invite(VoiceRoomInviteStatus::Accepted, Some(old_server_resolution)),
        local_now,
    );

    assert_eq!(deadline, Some(local_now + Duration::from_secs(10)));
    assert_eq!(
        results["invite-1"].invite.status,
        VoiceRoomInviteStatus::Accepted
    );
}

#[test]
fn recovered_result_uses_only_unexpired_server_window() {
    let resolved = time("2026-10-31T15:00:20Z");
    let terminal = invite(VoiceRoomInviteStatus::Accepted, Some(resolved));

    assert_eq!(
        remaining_recovered_result_ms(&terminal, Some(time("2026-10-31T15:00:29Z"))),
        1_000
    );
    assert_eq!(
        remaining_recovered_result_ms(&terminal, Some(time("2026-10-31T15:01:00Z"))),
        0
    );
    // Старый сервер не передаёт опорное время: сохраняем совместимое поведение.
    assert_eq!(remaining_recovered_result_ms(&terminal, None), 10_000);
}

#[test]
fn recovered_authoritative_expiry_removes_stale_local_unanswered_result() {
    let now = Instant::now();
    let mut results = HashMap::new();
    update_recent_result(
        &mut results,
        invite(VoiceRoomInviteStatus::Expired, None),
        now,
    );

    let resolved = invite(
        VoiceRoomInviteStatus::Expired,
        Some(time("2026-10-31T15:00:45Z")),
    );
    let deadline = update_recent_result_with_duration(
        &mut results,
        resolved,
        now + Duration::from_secs(1),
        Duration::ZERO,
    );

    assert!(deadline.is_none());
    assert!(results.is_empty());
}

#[test]
fn recovered_authoritative_result_shortens_existing_display_window() {
    let now = Instant::now();
    let mut results = HashMap::new();
    update_recent_result(
        &mut results,
        invite(VoiceRoomInviteStatus::Accepted, None),
        now,
    );

    let resolved = invite(
        VoiceRoomInviteStatus::Accepted,
        Some(time("2026-10-31T15:00:20Z")),
    );
    let deadline = update_recent_result_with_duration(
        &mut results,
        resolved,
        now + Duration::from_secs(1),
        Duration::from_secs(2),
    );

    assert_eq!(deadline, Some(now + Duration::from_secs(3)));
    assert_eq!(results["invite-1"].show_until, now + Duration::from_secs(3));
}

#[test]
fn late_authoritative_event_does_not_reopen_expired_local_result() {
    let now = Instant::now();
    let mut results = HashMap::new();
    let provisional = invite(VoiceRoomInviteStatus::Expired, None);
    update_recent_result(&mut results, provisional.clone(), now);
    assert!(remove_due_result(
        &mut results,
        "invite-1",
        now + Duration::from_secs(10),
    ));

    assert!(!should_remember_terminal_event(
        Some(&provisional),
        &results,
        "invite-1",
        now + Duration::from_secs(25),
    ));
}

#[test]
fn authoritative_correction_can_replace_still_visible_local_timeout() {
    let now = Instant::now();
    let mut results = HashMap::new();
    let provisional = invite(VoiceRoomInviteStatus::Expired, None);
    update_recent_result(&mut results, provisional.clone(), now);

    assert!(should_remember_terminal_event(
        Some(&provisional),
        &results,
        "invite-1",
        now + Duration::from_secs(1),
    ));
    assert!(should_remember_terminal_event(
        None,
        &results,
        "invite-new",
        now + Duration::from_secs(1),
    ));
}

#[test]
fn repeated_event_does_not_extend_result_display_time() {
    let now = Instant::now();
    let mut results = HashMap::new();
    let first = invite(VoiceRoomInviteStatus::Declined, None);
    let original_deadline = update_recent_result(&mut results, first.clone(), now);

    let duplicate_deadline =
        update_recent_result(&mut results, first, now + Duration::from_secs(7));

    assert_eq!(duplicate_deadline, original_deadline);
}

#[test]
fn old_timer_does_not_remove_result_with_later_deadline() {
    let now = Instant::now();
    let mut results = HashMap::new();
    update_recent_result(
        &mut results,
        invite(VoiceRoomInviteStatus::Expired, None),
        now,
    );
    let authoritative_resolution = time("2026-10-31T15:00:40Z");
    update_recent_result(
        &mut results,
        invite(
            VoiceRoomInviteStatus::Declined,
            Some(authoritative_resolution),
        ),
        now + Duration::from_secs(1),
    );

    assert!(!remove_due_result(
        &mut results,
        "invite-1",
        now + Duration::from_secs(10),
    ));
    assert_eq!(
        results["invite-1"].invite.status,
        VoiceRoomInviteStatus::Declined
    );
    assert!(remove_due_result(
        &mut results,
        "invite-1",
        now + Duration::from_secs(11),
    ));
    assert!(results.is_empty());
}

#[test]
fn monotonic_result_timer_rounds_up_fractional_milliseconds() {
    let now = Instant::now();
    assert_eq!(remaining_ms(now + Duration::from_nanos(1), now), 1);
    assert_eq!(remaining_ms(now + Duration::from_millis(1), now), 1);
    assert_eq!(
        remaining_ms(
            now + Duration::from_millis(1) + Duration::from_nanos(1),
            now
        ),
        2
    );
    assert_eq!(remaining_ms(now, now), 0);
}

#[test]
fn recovery_selects_only_own_terminal_invites_including_accepted() {
    let now = time("2026-10-31T15:00:45Z");
    let declined = invite(VoiceRoomInviteStatus::Declined, Some(now));
    let mut accepted = invite(VoiceRoomInviteStatus::Accepted, Some(now));
    accepted.invite_id = "invite-2".to_owned();
    let mut other_caller = invite(VoiceRoomInviteStatus::Cancelled, Some(now));
    other_caller.caller_user_id = "other-user".to_owned();
    let pending = invite(VoiceRoomInviteStatus::Pending, None);

    let selected =
        recovered_outgoing_resolutions(&[declined, accepted, other_caller, pending], "caller");

    assert_eq!(selected.len(), 2);
    assert!(
        selected
            .iter()
            .any(|item| item.status == VoiceRoomInviteStatus::Declined)
    );
    assert!(
        selected
            .iter()
            .any(|item| item.status == VoiceRoomInviteStatus::Accepted)
    );
}

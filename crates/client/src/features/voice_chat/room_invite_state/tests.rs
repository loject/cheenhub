//! Регрессии синхронизации приглашений между устройствами.

use std::collections::{HashMap, HashSet};
use std::time::Duration as StdDuration;

use cheenhub_contracts::realtime::{VoiceRoomInviteSnapshot, VoiceRoomInviteStatus};
use chrono::{DateTime, Duration, Utc};
use web_time::Instant;

use super::super::state::VoiceConnectionState;
use super::expiry::expiry_delay_for_invite;
use super::{
    VoiceRoomTarget, expire_pending_locally, expiry_delay_ms,
    incoming_invite as select_incoming_invite, reconcile_recovered_invites,
    should_join_after_accept, upsert_invite,
};

pub(super) fn invite(status: VoiceRoomInviteStatus) -> VoiceRoomInviteSnapshot {
    VoiceRoomInviteSnapshot {
        invite_id: "invite-1".to_owned(),
        server_id: "server-1".to_owned(),
        server_name: "Сервер".to_owned(),
        room_id: "room-1".to_owned(),
        room_name: "Голосовая".to_owned(),
        caller_user_id: "caller".to_owned(),
        caller_nickname: "Зовущий".to_owned(),
        caller_avatar_url: None,
        recipient_user_id: "recipient".to_owned(),
        recipient_nickname: None,
        recipient_avatar_url: None,
        status,
        created_at: "2026-10-31T15:00:00Z".to_owned(),
        expires_at: "2099-10-31T15:00:45Z".to_owned(),
        resolved_at: None,
    }
}

fn incoming_invite(
    invites: &[VoiceRoomInviteSnapshot],
    ignored_ids: &HashSet<String>,
    current_user_id: &str,
) -> Option<VoiceRoomInviteSnapshot> {
    let now = Instant::now();
    let deadlines = invites
        .iter()
        .filter_map(|invite| {
            let created = DateTime::parse_from_rfc3339(&invite.created_at).ok()?;
            let expires = DateTime::parse_from_rfc3339(&invite.expires_at).ok()?;
            let ttl_ms = expiry_delay_ms(expires.with_timezone(&Utc), created.with_timezone(&Utc));
            (ttl_ms > 0).then(|| {
                (
                    invite.invite_id.clone(),
                    now + StdDuration::from_millis(u64::from(ttl_ms)),
                )
            })
        })
        .collect::<HashMap<_, _>>();
    select_incoming_invite(invites, ignored_ids, current_user_id, &deadlines, now)
}

#[test]
fn invite_state_upserts_lifecycle_without_duplicate_ringing() {
    let mut invites = Vec::new();
    upsert_invite(&mut invites, invite(VoiceRoomInviteStatus::Pending));
    upsert_invite(&mut invites, invite(VoiceRoomInviteStatus::Accepted));
    assert_eq!(invites.len(), 1);
    assert_eq!(invites[0].status, VoiceRoomInviteStatus::Accepted);
    assert!(incoming_invite(&invites, &HashSet::new(), "recipient").is_none());
}

#[test]
fn invite_state_does_not_resurrect_terminal_after_delayed_send_ack() {
    for terminal_status in [
        VoiceRoomInviteStatus::Accepted,
        VoiceRoomInviteStatus::Declined,
        VoiceRoomInviteStatus::Expired,
        VoiceRoomInviteStatus::Cancelled,
    ] {
        let mut invites = vec![invite(terminal_status)];
        upsert_invite(&mut invites, invite(VoiceRoomInviteStatus::Pending));
        assert_eq!(invites.len(), 1);
        assert_eq!(invites[0].status, terminal_status);
        assert!(incoming_invite(&invites, &HashSet::new(), "recipient").is_none());
    }
}

#[test]
fn repeated_authoritative_terminal_does_not_create_another_result_event() {
    let mut terminal = invite(VoiceRoomInviteStatus::Expired);
    terminal.resolved_at = Some("2026-10-31T15:00:45Z".to_owned());
    let mut invites = vec![terminal.clone()];

    let applied = upsert_invite(&mut invites, terminal.clone());

    assert!(!applied);
    assert_eq!(invites, vec![terminal]);
}

#[test]
fn authoritative_terminal_replaces_local_timeout_only_once() {
    let mut provisional = invite(VoiceRoomInviteStatus::Pending);
    provisional.status = VoiceRoomInviteStatus::Expired;
    provisional.resolved_at = None;
    let mut invites = vec![provisional];
    let mut terminal = invite(VoiceRoomInviteStatus::Accepted);
    terminal.resolved_at = Some("2026-10-31T15:00:42Z".to_owned());

    assert!(upsert_invite(&mut invites, terminal.clone()));
    assert!(!upsert_invite(&mut invites, terminal.clone()));
    assert_eq!(invites, vec![terminal]);
}

#[test]
fn recovery_removes_missing_pending_but_keeps_events_received_during_request() {
    let old_pending = invite(VoiceRoomInviteStatus::Pending);
    let mut new_pending = invite(VoiceRoomInviteStatus::Pending);
    new_pending.invite_id = "invite-after-request".to_owned();
    let mut accepted = invite(VoiceRoomInviteStatus::Accepted);
    accepted.invite_id = "invite-accepted".to_owned();
    let mut invites = vec![old_pending, new_pending, accepted];
    let pending_at_request_start = HashSet::from(["invite-1".to_owned()]);

    reconcile_recovered_invites(&mut invites, vec![], &pending_at_request_start, "recipient");

    assert_eq!(invites.len(), 2);
    assert!(
        invites
            .iter()
            .any(|item| item.invite_id == "invite-after-request")
    );
    assert!(invites.iter().any(|item| {
        item.invite_id == "invite-accepted" && item.status == VoiceRoomInviteStatus::Accepted
    }));
}

#[test]
fn recovery_does_not_replace_terminal_state_with_stale_pending() {
    let mut invites = vec![invite(VoiceRoomInviteStatus::Accepted)];
    let pending_at_request_start = HashSet::new();

    reconcile_recovered_invites(
        &mut invites,
        vec![invite(VoiceRoomInviteStatus::Pending)],
        &pending_at_request_start,
        "recipient",
    );

    assert_eq!(invites.len(), 1);
    assert_eq!(invites[0].status, VoiceRoomInviteStatus::Accepted);
}

#[test]
fn local_expiry_timer_rounds_up_fractional_milliseconds() {
    let now = Utc::now();

    assert_eq!(expiry_delay_ms(now + Duration::microseconds(1), now), 1);
    assert_eq!(expiry_delay_ms(now + Duration::milliseconds(1), now), 1);
    assert_eq!(
        expiry_delay_ms(
            now + Duration::milliseconds(1) + Duration::microseconds(1),
            now
        ),
        2
    );
    assert_eq!(expiry_delay_ms(now - Duration::microseconds(1), now), 0);
    assert_eq!(expiry_delay_ms(now - Duration::seconds(1), now), 0);
}

#[test]
fn local_timer_marks_only_pending_invites_as_provisionally_expired() {
    let created = DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let expires = created + Duration::seconds(45);
    assert_eq!(expiry_delay_ms(expires, created), 45_000);

    let mut invites = vec![invite(VoiceRoomInviteStatus::Pending)];
    assert!(expire_pending_locally(&mut invites, "invite-1"));
    assert_eq!(invites.len(), 1);
    assert_eq!(invites[0].status, VoiceRoomInviteStatus::Expired);
    assert_eq!(invites[0].resolved_at, None);
    assert!(!expire_pending_locally(&mut invites, "invite-1"));

    // Опоздавший Pending не оживляет локально истёкшее приглашение.
    upsert_invite(&mut invites, invite(VoiceRoomInviteStatus::Pending));
    assert_eq!(invites[0].status, VoiceRoomInviteStatus::Expired);

    // Серверное решение остаётся авторитетнее предварительного локального тайм-аута.
    let mut authoritative = invite(VoiceRoomInviteStatus::Accepted);
    authoritative.resolved_at = Some("2026-10-31T15:00:42Z".to_owned());
    upsert_invite(&mut invites, authoritative);
    assert!(!expire_pending_locally(&mut invites, "invite-1"));
    assert_eq!(invites[0].status, VoiceRoomInviteStatus::Accepted);
}

#[test]
fn recovered_pending_uses_remaining_server_time_instead_of_full_ttl() {
    let mut pending = invite(VoiceRoomInviteStatus::Pending);
    pending.created_at = "2026-10-31T15:00:00Z".to_owned();
    pending.expires_at = "2026-10-31T15:00:45Z".to_owned();
    let server_now = DateTime::parse_from_rfc3339("2026-10-31T15:00:40Z")
        .unwrap()
        .with_timezone(&Utc);

    assert_eq!(expiry_delay_for_invite(&pending, Some(server_now)), 5_000);
    assert_eq!(expiry_delay_for_invite(&pending, None), 45_000);
}

#[test]
fn delayed_recovery_cannot_resurrect_invite_expired_while_request_was_in_flight() {
    let mut invites = vec![invite(VoiceRoomInviteStatus::Pending)];
    assert!(expire_pending_locally(&mut invites, "invite-1"));
    let pending_at_request_start = HashSet::from(["invite-1".to_owned()]);

    reconcile_recovered_invites(
        &mut invites,
        vec![invite(VoiceRoomInviteStatus::Pending)],
        &pending_at_request_start,
        "recipient",
    );

    assert_eq!(invites.len(), 1);
    assert_eq!(invites[0].status, VoiceRoomInviteStatus::Expired);
    assert_eq!(invites[0].resolved_at, None);
}

#[test]
fn accepted_invite_retries_join_after_error_or_disconnect_started() {
    let target = VoiceRoomTarget::server(
        "server-1".to_owned(),
        "room-1".to_owned(),
        "Голосовая".to_owned(),
    );
    let failed = VoiceConnectionState::Error {
        target: Some(target.clone()),
        message: "Ошибка входа".to_owned(),
    };
    let leaving = VoiceConnectionState::Disconnecting {
        target: target.clone(),
        participants: Vec::new(),
    };
    let connected = VoiceConnectionState::Connected {
        target: target.clone(),
        participants: Vec::new(),
    };

    assert!(should_join_after_accept(&failed, &target));
    assert!(should_join_after_accept(&leaving, &target));
    assert!(!should_join_after_accept(&connected, &target));
}

#[test]
fn pending_invite_rings_when_server_timestamps_are_behind_device_clock() {
    let mut pending = invite(VoiceRoomInviteStatus::Pending);
    pending.created_at = "2000-01-01T00:00:00Z".to_owned();
    pending.expires_at = "2000-01-01T00:00:45Z".to_owned();

    assert!(incoming_invite(&[pending], &HashSet::new(), "recipient").is_some());
}

#[test]
fn expired_local_deadline_does_not_ring_even_with_future_server_expiration() {
    let pending = invite(VoiceRoomInviteStatus::Pending);
    let now = Instant::now();
    let deadlines = HashMap::from([("invite-1".to_owned(), now - StdDuration::from_millis(1))]);

    assert!(
        select_incoming_invite(&[pending], &HashSet::new(), "recipient", &deadlines, now).is_none()
    );
}

#[test]
fn invite_ignore_hides_only_this_device_without_changing_server_state() {
    let invites = vec![invite(VoiceRoomInviteStatus::Pending)];
    let mut ignored = HashSet::new();
    assert!(incoming_invite(&invites, &ignored, "recipient").is_some());
    ignored.insert("invite-1".to_owned());
    assert!(incoming_invite(&invites, &ignored, "recipient").is_none());
    assert_eq!(invites[0].status, VoiceRoomInviteStatus::Pending);
    assert!(incoming_invite(&invites, &HashSet::new(), "recipient").is_some());
    assert!(incoming_invite(&invites, &HashSet::new(), "caller").is_none());
}

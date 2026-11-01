//! Тесты плиток исходящих приглашений в голосовую комнату.

use super::*;
use std::time::Duration;

fn invite(status: VoiceRoomInviteStatus) -> VoiceRoomInviteSnapshot {
    VoiceRoomInviteSnapshot {
        invite_id: "invite-1".into(),
        server_id: "server-1".into(),
        server_name: "Сервер".into(),
        room_id: "room-1".into(),
        room_name: "Голосовая".into(),
        caller_user_id: "caller".into(),
        caller_nickname: "Отправитель".into(),
        caller_avatar_url: None,
        recipient_user_id: "recipient".into(),
        recipient_nickname: None,
        recipient_avatar_url: None,
        status,
        created_at: "2026-10-31T15:00:00Z".into(),
        expires_at: "2026-10-31T15:00:45Z".into(),
        resolved_at: None,
    }
}

fn tiles(
    invites: &[VoiceRoomInviteSnapshot],
    results: &HashMap<String, RecentOutgoingResult>,
    present_users: &HashSet<String>,
    now: Instant,
) -> Vec<OutgoingInviteTile> {
    let deadlines = invites
        .iter()
        .filter(|invite| invite.status == VoiceRoomInviteStatus::Pending)
        .map(|invite| (invite.invite_id.clone(), now + Duration::from_secs(45)))
        .collect::<HashMap<_, _>>();
    let candidates = HashMap::from([(
        "recipient".to_owned(),
        VoiceRoomInviteCandidate {
            user_id: "recipient".into(),
            nickname: "Алиса".into(),
            avatar_url: Some("/avatar.png".into()),
        },
    )]);
    outgoing_invite_tiles(
        invites,
        &deadlines,
        results,
        &candidates,
        OutgoingInviteScope {
            caller_id: "caller",
            server_id: "server-1",
            room_id: "room-1",
        },
        present_users,
        now,
    )
}

#[test]
fn pending_invite_renders_a_named_call_tile_until_deadline() {
    let pending = invite(VoiceRoomInviteStatus::Pending);
    let now = Instant::now();
    let items = tiles(
        std::slice::from_ref(&pending),
        &HashMap::new(),
        &HashSet::new(),
        now,
    );
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].nickname, "Алиса");
    assert_eq!(items[0].avatar_url.as_deref(), Some("/avatar.png"));
    assert_eq!(items[0].status, OutgoingInviteTileStatus::Calling);
    let deadlines = HashMap::from([("invite-1".to_owned(), now + Duration::from_secs(45))]);
    assert!(
        outgoing_invite_tiles(
            &[pending],
            &deadlines,
            &HashMap::new(),
            &HashMap::new(),
            OutgoingInviteScope {
                caller_id: "caller",
                server_id: "server-1",
                room_id: "room-1"
            },
            &HashSet::new(),
            now + Duration::from_secs(45),
        )
        .is_empty()
    );
}

#[test]
fn pending_call_tile_survives_server_clock_being_behind_device_clock() {
    let mut pending = invite(VoiceRoomInviteStatus::Pending);
    pending.expires_at = "2000-01-01T00:00:45Z".into();

    let items = tiles(&[pending], &HashMap::new(), &HashSet::new(), Instant::now());

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].status, OutgoingInviteTileStatus::Calling);
}

#[test]
fn accepted_invite_shows_waiting_tile_until_participant_joins_or_ten_seconds_pass() {
    let now = Instant::now();
    let result = RecentOutgoingResult {
        invite: invite(VoiceRoomInviteStatus::Accepted),
        show_until: now + Duration::from_secs(10),
    };
    let results = HashMap::from([("invite-1".into(), result)]);

    let waiting = tiles(&[], &results, &HashSet::new(), now + Duration::from_secs(9));
    assert_eq!(waiting.len(), 1);
    assert_eq!(waiting[0].status, OutgoingInviteTileStatus::AcceptedWaiting);
    assert!(tiles(&[], &results, &HashSet::from(["recipient".into()]), now).is_empty());
    assert!(
        tiles(
            &[],
            &results,
            &HashSet::new(),
            now + Duration::from_secs(10)
        )
        .is_empty()
    );
}

#[test]
fn timeout_and_decline_remain_visible_for_ten_seconds_only() {
    let now = Instant::now();
    for (status, expected) in [
        (
            VoiceRoomInviteStatus::Expired,
            OutgoingInviteTileStatus::Unanswered,
        ),
        (
            VoiceRoomInviteStatus::Declined,
            OutgoingInviteTileStatus::Declined,
        ),
    ] {
        let result = RecentOutgoingResult {
            invite: invite(status),
            show_until: now + Duration::from_secs(10),
        };
        let results = HashMap::from([("invite-1".into(), result)]);
        let items = tiles(&[], &results, &HashSet::new(), now + Duration::from_secs(9));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status, expected);
        assert!(
            tiles(
                &[],
                &results,
                &HashSet::new(),
                now + Duration::from_secs(10)
            )
            .is_empty()
        );
    }
}

#[test]
fn cancelled_invite_stays_visible_for_ten_seconds() {
    let now = Instant::now();
    let result = RecentOutgoingResult {
        invite: invite(VoiceRoomInviteStatus::Cancelled),
        show_until: now + Duration::from_secs(10),
    };
    let results = HashMap::from([("invite-1".into(), result)]);
    let items = tiles(&[], &results, &HashSet::new(), now + Duration::from_secs(9));
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].nickname, "Алиса");
    assert_eq!(items[0].status, OutgoingInviteTileStatus::Cancelled);
    assert!(
        tiles(
            &[],
            &results,
            &HashSet::new(),
            now + Duration::from_secs(10)
        )
        .is_empty()
    );
}

#[test]
fn joined_users_and_other_rooms_do_not_receive_placeholder_tiles() {
    let pending = invite(VoiceRoomInviteStatus::Pending);
    let now = Instant::now();
    assert!(
        tiles(
            std::slice::from_ref(&pending),
            &HashMap::new(),
            &HashSet::from(["recipient".into()]),
            now
        )
        .is_empty()
    );
    let mut other_room = pending;
    other_room.room_id = "room-2".into();
    assert!(tiles(&[other_room], &HashMap::new(), &HashSet::new(), now).is_empty());
}

#[test]
fn recovered_invite_displays_recipient_identity_without_candidate_cache() {
    let mut pending = invite(VoiceRoomInviteStatus::Pending);
    pending.recipient_nickname = Some("Аня".to_owned());
    pending.recipient_avatar_url = Some("/avatars/anna.png".to_owned());
    let now = Instant::now();
    let deadlines = HashMap::from([("invite-1".to_owned(), now + Duration::from_secs(45))]);

    let items = outgoing_invite_tiles(
        &[pending],
        &deadlines,
        &HashMap::new(),
        &HashMap::new(),
        OutgoingInviteScope {
            caller_id: "caller",
            server_id: "server-1",
            room_id: "room-1",
        },
        &HashSet::new(),
        now,
    );

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].nickname, "Аня");
    assert_eq!(items[0].avatar_url.as_deref(), Some("/avatars/anna.png"));
}

#[test]
fn unknown_recipient_does_not_expose_internal_user_identifier() {
    let mut pending = invite(VoiceRoomInviteStatus::Pending);
    pending.recipient_user_id = "12345678-1234-1234-1234-123456789abc".into();
    let now = Instant::now();
    let deadlines = HashMap::from([("invite-1".to_owned(), now + Duration::from_secs(45))]);
    let items = outgoing_invite_tiles(
        &[pending],
        &deadlines,
        &HashMap::new(),
        &HashMap::new(),
        OutgoingInviteScope {
            caller_id: "caller",
            server_id: "server-1",
            room_id: "room-1",
        },
        &HashSet::new(),
        now,
    );

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].nickname, "Приглашённый участник");
}

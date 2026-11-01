//! Тесты атомарных переходов приглашений в серверную голосовую комнату.

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::{InMemoryVoiceRoomInviteStore, VoiceRoomInvite, VoiceRoomInviteStoreError};
use cheenhub_contracts::realtime::{VoiceRoomInviteResponse, VoiceRoomInviteStatus};

fn invite(now: DateTime<Utc>) -> VoiceRoomInvite {
    VoiceRoomInvite {
        id: Uuid::new_v4(),
        server_id: Uuid::new_v4(),
        server_name: "Сервер".to_owned(),
        room_id: Uuid::new_v4(),
        room_name: "Голосовая".to_owned(),
        caller_user_id: Uuid::new_v4(),
        caller_nickname: "Отправитель".to_owned(),
        caller_avatar_url: None,
        recipient_user_id: Uuid::new_v4(),
        recipient_nickname: "Получатель".to_owned(),
        recipient_avatar_url: Some("/avatars/recipient.png".to_owned()),
        status: VoiceRoomInviteStatus::Pending,
        created_at: now,
        expires_at: now + Duration::seconds(45),
        resolved_at: None,
    }
}

#[tokio::test]
async fn voice_invite_store_blocks_repeated_pending_and_isolates_user_lists() {
    let store = InMemoryVoiceRoomInviteStore::default();
    let now = Utc::now();
    let first = invite(now);
    store.start(first.clone(), now).await.unwrap();

    assert!(matches!(
        store.start(first.clone(), now).await,
        Err(VoiceRoomInviteStoreError::AlreadyPending)
    ));
    assert_eq!(store.list_for_user(&first.caller_user_id).await.len(), 1);
    assert_eq!(store.list_for_user(&first.recipient_user_id).await.len(), 1);
    assert!(store.list_for_user(&Uuid::new_v4()).await.is_empty());
}

#[tokio::test]
async fn voice_invite_store_snapshot_preserves_recipient_metadata_and_resolution_time() {
    let store = InMemoryVoiceRoomInviteStore::default();
    let now = Utc::now();
    let first = invite(now);
    store.start(first.clone(), now).await.unwrap();

    let pending = store.find(&first.id).await.unwrap().snapshot();
    assert_eq!(pending.recipient_nickname.as_deref(), Some("Получатель"));
    assert_eq!(
        pending.recipient_avatar_url.as_deref(),
        Some("/avatars/recipient.png")
    );
    assert!(pending.resolved_at.is_none());

    let resolved_at = now + Duration::seconds(3);
    let resolved = store
        .respond(
            &first.id,
            &first.recipient_user_id,
            VoiceRoomInviteResponse::Decline,
            resolved_at,
        )
        .await
        .unwrap();
    let restored = resolved.snapshot();
    assert_eq!(restored.status, VoiceRoomInviteStatus::Declined);
    assert_eq!(
        restored.resolved_at.as_deref(),
        Some(resolved_at.to_rfc3339().as_str())
    );
    assert_eq!(restored.recipient_nickname.as_deref(), Some("Получатель"));
}

#[tokio::test]
async fn voice_invite_store_rejects_immediate_resend_after_decline_but_allows_retry_after_cooldown()
{
    let store = InMemoryVoiceRoomInviteStore::default();
    let now = Utc::now();
    let first = invite(now);
    store.start(first.clone(), now).await.unwrap();
    store
        .respond(
            &first.id,
            &first.recipient_user_id,
            VoiceRoomInviteResponse::Decline,
            now + Duration::seconds(1),
        )
        .await
        .unwrap();

    let mut retry = first.clone();
    retry.id = Uuid::new_v4();
    retry.room_id = Uuid::new_v4();
    retry.created_at = now + Duration::seconds(2);
    retry.expires_at = retry.created_at + Duration::seconds(45);
    assert!(matches!(
        store.start(retry.clone(), retry.created_at).await,
        Err(VoiceRoomInviteStoreError::TooSoon)
    ));

    retry.created_at = now + Duration::seconds(30);
    retry.expires_at = retry.created_at + Duration::seconds(45);
    store
        .start(retry, now + Duration::seconds(30))
        .await
        .unwrap();
}

#[tokio::test]
async fn voice_invite_store_caps_simultaneous_pending_invites_to_one_recipient() {
    let store = InMemoryVoiceRoomInviteStore::default();
    let now = Utc::now();
    let recipient = Uuid::new_v4();
    let mut first = None;
    for _ in 0..5 {
        let mut current = invite(now);
        current.recipient_user_id = recipient;
        store.start(current.clone(), now).await.unwrap();
        if first.is_none() {
            first = Some(current);
        }
    }

    let mut overflow = invite(now);
    overflow.recipient_user_id = recipient;
    assert!(matches!(
        store.start(overflow.clone(), now).await,
        Err(VoiceRoomInviteStoreError::RecipientAtCapacity)
    ));

    let first = first.unwrap();
    store
        .cancel(&first.id, &first.caller_user_id, now)
        .await
        .unwrap();
    store.start(overflow, now).await.unwrap();
}

#[tokio::test]
async fn voice_invite_store_accepts_only_once_across_concurrent_devices() {
    let store = InMemoryVoiceRoomInviteStore::default();
    let now = Utc::now();
    let first = invite(now);
    store.start(first.clone(), now).await.unwrap();

    let (a, b) = tokio::join!(
        store.respond(
            &first.id,
            &first.recipient_user_id,
            VoiceRoomInviteResponse::Accept,
            now
        ),
        store.respond(
            &first.id,
            &first.recipient_user_id,
            VoiceRoomInviteResponse::Accept,
            now
        ),
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert_eq!(
        store.list_for_user(&first.recipient_user_id).await[0].status,
        VoiceRoomInviteStatus::Accepted
    );
    assert!(matches!(
        store.cancel(&first.id, &first.caller_user_id, now).await,
        Err(VoiceRoomInviteStoreError::InvalidState)
    ));
}

#[tokio::test]
async fn voice_invite_store_expires_at_exact_deadline_and_allows_retry() {
    let store = InMemoryVoiceRoomInviteStore::default();
    let now = Utc::now();
    let first = invite(now);
    store.start(first.clone(), now).await.unwrap();
    let deadline = now + Duration::seconds(45);

    assert!(matches!(
        store
            .respond(
                &first.id,
                &first.recipient_user_id,
                VoiceRoomInviteResponse::Accept,
                deadline
            )
            .await,
        Err(VoiceRoomInviteStoreError::Expired(_))
    ));
    assert_eq!(
        store.list_for_user(&first.caller_user_id).await[0].status,
        VoiceRoomInviteStatus::Expired
    );
    let mut retry = first.clone();
    retry.id = Uuid::new_v4();
    retry.created_at = deadline;
    retry.expires_at = deadline + Duration::seconds(45);
    store.start(retry, deadline).await.unwrap();
}

#[tokio::test]
async fn voice_invite_store_authorizes_respondent_and_cancellation() {
    let store = InMemoryVoiceRoomInviteStore::default();
    let now = Utc::now();
    let first = invite(now);
    store.start(first.clone(), now).await.unwrap();
    let stranger = Uuid::new_v4();

    assert!(matches!(
        store
            .respond(&first.id, &stranger, VoiceRoomInviteResponse::Decline, now)
            .await,
        Err(VoiceRoomInviteStoreError::Unauthorized)
    ));
    assert!(matches!(
        store.cancel(&first.id, &stranger, now).await,
        Err(VoiceRoomInviteStoreError::Unauthorized)
    ));
    let cancelled = store
        .cancel(&first.id, &first.caller_user_id, now)
        .await
        .unwrap();
    assert_eq!(cancelled.status, VoiceRoomInviteStatus::Cancelled);
    assert!(matches!(
        store
            .respond(
                &first.id,
                &first.recipient_user_id,
                VoiceRoomInviteResponse::Accept,
                now
            )
            .await,
        Err(VoiceRoomInviteStoreError::InvalidState)
    ));
}

#[tokio::test]
async fn voice_invite_store_discards_resolved_records_after_retention_on_list() {
    let store = InMemoryVoiceRoomInviteStore::default();
    let old = Utc::now() - Duration::minutes(5);
    let first = invite(old);
    store.start(first.clone(), old).await.unwrap();
    store
        .respond(
            &first.id,
            &first.recipient_user_id,
            VoiceRoomInviteResponse::Decline,
            old + Duration::seconds(1),
        )
        .await
        .unwrap();
    assert!(store.list_for_user(&first.caller_user_id).await.is_empty());

    let now = Utc::now();
    let recent = invite(now);
    store.start(recent.clone(), now).await.unwrap();
    store
        .respond(
            &recent.id,
            &recent.recipient_user_id,
            VoiceRoomInviteResponse::Accept,
            now,
        )
        .await
        .unwrap();
    assert_eq!(store.list_for_user(&recent.caller_user_id).await.len(), 1);
}

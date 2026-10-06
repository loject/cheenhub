//! Проверки расчёта собственных ссылок приглашения.

use super::{active_link_count, invite_link};
use crate::features::servers::domain::ServerInvite;
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

fn invite(max_uses: Option<u32>, expires_at: Option<DateTime<Utc>>) -> ServerInvite {
    ServerInvite {
        id: Uuid::new_v4(),
        server_id: Uuid::new_v4(),
        creator_user_id: Uuid::new_v4(),
        max_uses,
        expires_at,
        created_at: Utc::now(),
        revoked_at: None,
        deleted_at: None,
    }
}

#[test]
fn counts_only_active_links_towards_the_limit() {
    let now = Utc::now();
    let active = invite_link(&invite(None, None), None, now);
    let expired = invite_link(&invite(None, Some(now - Duration::days(1))), None, now);
    let exhausted = invite_link(&invite(Some(1), None), Some(&1), now);

    assert!(active.is_active);
    assert!(!expired.is_active);
    assert!(!exhausted.is_active);
    assert_eq!(active_link_count(&[active, expired, exhausted]), 1);
}

#[test]
fn treats_revoked_invite_as_inactive() {
    let now = Utc::now();
    let mut revoked = invite(None, None);
    revoked.revoked_at = Some(now);

    assert!(!invite_link(&revoked, None, now).is_active);
}

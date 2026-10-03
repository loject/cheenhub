use super::{OwnInviteLink, OwnInviteLinks};

fn link(is_active: bool) -> OwnInviteLink {
    OwnInviteLink {
        code: "code".to_owned(),
        created_at: "2026-01-01T00:00:00+00:00".to_owned(),
        expires_at: None,
        max_uses: None,
        uses: 0,
        is_active,
    }
}

fn links(limit: Option<u32>, active: usize, inactive: usize) -> OwnInviteLinks {
    let mut all = (0..active).map(|_| link(true)).collect::<Vec<_>>();
    all.extend((0..inactive).map(|_| link(false)));

    OwnInviteLinks { links: all, limit }
}

#[test]
fn counts_only_active_links() {
    assert_eq!(links(Some(3), 2, 4).active_count(), 2);
}

#[test]
fn reports_limit_reached_only_when_active_count_matches_limit() {
    assert!(!links(Some(3), 2, 0).is_limit_reached());
    assert!(links(Some(3), 3, 0).is_limit_reached());
    assert!(links(Some(3), 4, 0).is_limit_reached());
}

#[test]
fn keeps_creating_allowed_when_only_inactive_links_exist() {
    assert!(!links(Some(1), 0, 5).is_limit_reached());
}

#[test]
fn never_reports_limit_reached_without_a_limit() {
    assert!(!links(None, 0, 0).is_limit_reached());
    assert!(!links(None, 50, 50).is_limit_reached());
}

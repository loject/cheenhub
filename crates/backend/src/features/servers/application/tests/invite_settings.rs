use cheenhub_contracts::realtime::{KickServerInviteMember, ListServerInvites, RevokeServerInvite};

use super::*;

#[tokio::test]
async fn owner_can_list_server_invites_with_joined_members() {
    let state = state();
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invite_settings_owner".to_owned(),
            email: "invite-settings-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invite_settings_guest".to_owned(),
            email: "invite-settings-guest@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("guest registration should succeed");
    let server = create(
        &state,
        &owner_auth.access_token,
        CreateServerRequest {
            name: "Invite Settings".to_owned(),
        },
    )
    .await
    .expect("server should be created");
    let invite = create_invite(
        &state,
        &owner_auth.access_token,
        server.server.id.clone(),
        CreateServerInviteRequest {
            max_uses: Some(4),
            expires_in_days: Some(7),
        },
    )
    .await
    .expect("invite should be created");
    accept_invite(&state, &guest_auth.access_token, invite.code.clone())
        .await
        .expect("guest should join");
    let owner_id = Uuid::parse_str(&owner_auth.user.id).expect("owner id should be uuid");

    let response = list_server_invites(
        &state,
        &owner_id,
        ListServerInvites {
            server_id: server.server.id,
        },
    )
    .await
    .expect("invites should load");

    assert_eq!(response.invites.len(), 1);
    assert_eq!(response.invites[0].code, invite.code);
    assert_eq!(
        response.invites[0].author_nickname,
        owner_auth.user.nickname
    );
    assert_eq!(response.invites[0].uses, 1);
    assert_eq!(response.invites[0].joined_members.len(), 1);
    assert_eq!(
        response.invites[0].joined_members[0].nickname,
        guest_auth.user.nickname
    );
    assert!(response.invites[0].joined_members[0].is_active_member);
}

#[tokio::test]
async fn non_owner_cannot_manage_server_invites() {
    let state = state();
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invite_settings_real_owner".to_owned(),
            email: "invite-settings-real-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invite_settings_not_owner".to_owned(),
            email: "invite-settings-not-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("guest registration should succeed");
    let server = create(
        &state,
        &owner_auth.access_token,
        CreateServerRequest {
            name: "Invite Permissions".to_owned(),
        },
    )
    .await
    .expect("server should be created");
    let guest_id = Uuid::parse_str(&guest_auth.user.id).expect("guest id should be uuid");

    let error = list_server_invites(
        &state,
        &guest_id,
        ListServerInvites {
            server_id: server.server.id,
        },
    )
    .await
    .expect_err("non-owner invite list should fail");

    assert!(matches!(error, ServerError::NotFound(_)));
}

#[tokio::test]
async fn revoked_invite_cannot_be_loaded_or_accepted() {
    let state = state();
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invite_revoke_owner".to_owned(),
            email: "invite-revoke-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invite_revoke_guest".to_owned(),
            email: "invite-revoke-guest@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("guest registration should succeed");
    let server = create(
        &state,
        &owner_auth.access_token,
        CreateServerRequest {
            name: "Invite Revoke".to_owned(),
        },
    )
    .await
    .expect("server should be created");
    let invite = create_invite(
        &state,
        &owner_auth.access_token,
        server.server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("invite should be created");
    let owner_id = Uuid::parse_str(&owner_auth.user.id).expect("owner id should be uuid");
    let revoked = revoke_server_invite(
        &state,
        &owner_id,
        RevokeServerInvite {
            server_id: server.server.id,
            code: invite.code.clone(),
        },
    )
    .await
    .expect("invite should be revoked");

    assert_eq!(revoked.code, invite.code);
    assert!(
        invite_info(&state, &owner_auth.access_token, invite.code.clone())
            .await
            .is_err()
    );
    assert!(
        accept_invite(&state, &guest_auth.access_token, invite.code)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn kicked_invite_member_stays_in_invite_history() {
    let state = state();
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invite_kick_owner".to_owned(),
            email: "invite-kick-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invite_kick_guest".to_owned(),
            email: "invite-kick-guest@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("guest registration should succeed");
    let server = create(
        &state,
        &owner_auth.access_token,
        CreateServerRequest {
            name: "Invite Kick".to_owned(),
        },
    )
    .await
    .expect("server should be created");
    let invite = create_invite(
        &state,
        &owner_auth.access_token,
        server.server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("invite should be created");
    accept_invite(&state, &guest_auth.access_token, invite.code.clone())
        .await
        .expect("guest should join");
    let owner_id = Uuid::parse_str(&owner_auth.user.id).expect("owner id should be uuid");
    kick_server_invite_member(
        &state,
        &owner_id,
        KickServerInviteMember {
            server_id: server.server.id.clone(),
            invite_code: invite.code,
            user_id: guest_auth.user.id.clone(),
        },
    )
    .await
    .expect("guest should be kicked");

    let response = list_server_invites(
        &state,
        &owner_id,
        ListServerInvites {
            server_id: server.server.id,
        },
    )
    .await
    .expect("invites should load");

    assert_eq!(response.invites[0].joined_members.len(), 1);
    assert!(!response.invites[0].joined_members[0].is_active_member);
}

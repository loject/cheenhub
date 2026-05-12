use super::*;

#[tokio::test]
async fn owner_can_create_server_invite() {
    let server_store = Arc::new(InMemoryServerStore::default());
    let state = state_with_store(server_store.clone());
    let auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invite_owner".to_owned(),
            email: "invite-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed");
    let server = create(
        &state,
        &auth.access_token,
        CreateServerRequest {
            name: "Invite Hub".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");

    let response = create_invite(
        &state,
        &auth.access_token,
        server.server.id.clone(),
        CreateServerInviteRequest {
            max_uses: Some(5),
            expires_in_days: Some(3),
        },
    )
    .await
    .expect("invite creation should succeed");
    let invites = server_store
        .invites_for_tests()
        .expect("invites should be readable");

    assert_eq!(invites.len(), 1);
    assert_eq!(response.code, invites[0].id.to_string());
    assert_eq!(invites[0].server_id.to_string(), server.server.id);
    assert_eq!(invites[0].creator_user_id.to_string(), auth.user.id);
    assert_eq!(invites[0].max_uses, Some(5));
    assert!(invites[0].expires_at.is_some());
    assert!(invites[0].created_at <= chrono::Utc::now());
}

#[tokio::test]
async fn owner_can_load_server_invite_info() {
    let state = state();
    let auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "info_owner".to_owned(),
            email: "info-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed");
    let server = create(
        &state,
        &auth.access_token,
        CreateServerRequest {
            name: "Info Hub".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
    let invite = create_invite(
        &state,
        &auth.access_token,
        server.server.id.clone(),
        CreateServerInviteRequest {
            max_uses: Some(7),
            expires_in_days: Some(5),
        },
    )
    .await
    .expect("invite creation should succeed");

    let response = invite_info(&state, &auth.access_token, invite.code.clone())
        .await
        .expect("invite info should load");

    assert_eq!(response.invite.code, invite.code);
    assert_eq!(response.invite.uses, 0);
    assert_eq!(response.invite.max_uses, Some(7));
    assert!(response.invite.expires_at.is_some());
    assert_eq!(response.server.id, server.server.id);
    assert_eq!(response.server.name, "Info Hub");
    assert!(response.server.is_owner);
    assert!(response.server.is_member);
}

#[tokio::test]
async fn non_owner_can_load_server_invite_info() {
    let state = state();
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "info_owner_two".to_owned(),
            email: "info-owner-two@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "info_guest".to_owned(),
            email: "info-guest@example.com".to_owned(),
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
            name: "Shared Info".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
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
    .expect("invite creation should succeed");

    let response = invite_info(&state, &guest_auth.access_token, invite.code)
        .await
        .expect("invite info should load for another user");

    assert_eq!(response.server.id, server.server.id);
    assert!(!response.server.is_owner);
    assert!(!response.server.is_member);
}

#[tokio::test]
async fn non_member_accepts_invite_and_server_appears_in_list() {
    let server_store = Arc::new(InMemoryServerStore::default());
    let state = state_with_store(server_store.clone());
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "accept_owner".to_owned(),
            email: "accept-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "accept_guest".to_owned(),
            email: "accept-guest@example.com".to_owned(),
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
            name: "Joinable".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
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
    .expect("invite creation should succeed");

    let accepted = accept_invite(&state, &guest_auth.access_token, invite.code.clone())
        .await
        .expect("invite should be accepted");
    let listed = list(&state, &guest_auth.access_token)
        .await
        .expect("joined server list should load");
    let invite_info = invite_info(&state, &guest_auth.access_token, invite.code)
        .await
        .expect("invite info should load");
    let invite_uses = server_store
        .invite_uses_for_tests()
        .expect("invite uses should be readable");

    assert!(!accepted.already_member);
    assert_eq!(accepted.server.id, server.server.id);
    assert!(!accepted.server.is_owner);
    assert!(accepted.server.is_member);
    assert_eq!(listed.servers, vec![accepted.server]);
    assert_eq!(invite_info.invite.uses, 1);
    assert_eq!(invite_uses.len(), 1);
    assert_eq!(invite_uses[0].user_id.to_string(), guest_auth.user.id);
}

#[tokio::test]
async fn member_can_leave_joined_server() {
    let state = state();
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "leave_owner".to_owned(),
            email: "leave-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "leave_guest".to_owned(),
            email: "leave-guest@example.com".to_owned(),
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
            name: "Leavable".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
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
    .expect("invite creation should succeed");
    accept_invite(&state, &guest_auth.access_token, invite.code)
        .await
        .expect("invite should be accepted");

    leave(&state, &guest_auth.access_token, server.server.id)
        .await
        .expect("member should leave");
    let listed = list(&state, &guest_auth.access_token)
        .await
        .expect("server list should load after leaving");

    assert!(listed.servers.is_empty());
}

#[tokio::test]
async fn owner_cannot_leave_owned_server() {
    let state = state();
    let auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "leave_blocked_owner".to_owned(),
            email: "leave-blocked-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed");
    let server = create(
        &state,
        &auth.access_token,
        CreateServerRequest {
            name: "Owned".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");

    let error = leave(&state, &auth.access_token, server.server.id)
        .await
        .expect_err("owner leave should fail");

    assert!(matches!(error, ServerError::BadRequest(_)));
}

#[tokio::test]
async fn active_member_accept_returns_already_member_without_new_usage() {
    let server_store = Arc::new(InMemoryServerStore::default());
    let state = state_with_store(server_store.clone());
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "already_owner".to_owned(),
            email: "already-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let server = create(
        &state,
        &owner_auth.access_token,
        CreateServerRequest {
            name: "Already".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
    let invite = create_invite(
        &state,
        &owner_auth.access_token,
        server.server.id,
        CreateServerInviteRequest {
            max_uses: Some(1),
            expires_in_days: None,
        },
    )
    .await
    .expect("invite creation should succeed");

    let accepted = accept_invite(&state, &owner_auth.access_token, invite.code)
        .await
        .expect("owner should already be a member");
    let invite_uses = server_store
        .invite_uses_for_tests()
        .expect("invite uses should be readable");

    assert!(accepted.already_member);
    assert!(accepted.server.is_owner);
    assert!(accepted.server.is_member);
    assert!(invite_uses.is_empty());
}

#[tokio::test]
async fn accept_invite_accepts_compact_uuid_code() {
    let state = state();
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "compact_accept_owner".to_owned(),
            email: "compact-accept-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "compact_accept_guest".to_owned(),
            email: "compact-accept-guest@example.com".to_owned(),
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
            name: "Compact Accept".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
    let invite = create_invite(
        &state,
        &owner_auth.access_token,
        server.server.id,
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("invite creation should succeed");

    let response = accept_invite(
        &state,
        &guest_auth.access_token,
        invite.code.replace('-', ""),
    )
    .await
    .expect("compact invite should be accepted");

    assert_eq!(response.server.name, "Compact Accept");
    assert!(response.server.is_member);
}

#[tokio::test]
async fn invite_info_accepts_compact_uuid_code() {
    let state = state();
    let auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "compact_owner".to_owned(),
            email: "compact-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed");
    let server = create(
        &state,
        &auth.access_token,
        CreateServerRequest {
            name: "Compact".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
    let invite = create_invite(
        &state,
        &auth.access_token,
        server.server.id,
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("invite creation should succeed");
    let compact_code = invite.code.replace('-', "");

    let response = invite_info(&state, &auth.access_token, compact_code)
        .await
        .expect("compact invite code should load");

    assert_eq!(response.invite.code, invite.code);
}

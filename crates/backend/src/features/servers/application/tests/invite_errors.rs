use super::*;

#[tokio::test]
async fn invite_info_rejects_missing_invalid_and_expired_invites() {
    let server_store = Arc::new(InMemoryServerStore::default());
    let state = state_with_store(server_store.clone());
    let auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invite_error_owner".to_owned(),
            email: "invite-error-owner@example.com".to_owned(),
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
            name: "Expired".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
    let owner_user_id = Uuid::parse_str(&auth.user.id).expect("user id should be uuid");
    let server_id = Uuid::parse_str(&server.server.id).expect("server id should be uuid");
    let expired_invite = server_store
        .insert_server_invite(
            &server_id,
            &owner_user_id,
            None,
            Some(chrono::Utc::now() - chrono::Duration::days(1)),
        )
        .await
        .expect("expired invite should be inserted");

    let invalid = invite_info(&state, &auth.access_token, "not-a-uuid".to_owned())
        .await
        .expect_err("invalid invite code should fail");
    let missing = invite_info(&state, &auth.access_token, Uuid::new_v4().to_string())
        .await
        .expect_err("missing invite should fail");
    let expired = invite_info(&state, &auth.access_token, expired_invite.id.to_string())
        .await
        .expect_err("expired invite should fail");

    assert!(matches!(invalid, ServerError::BadRequest(_)));
    assert!(matches!(missing, ServerError::NotFound(_)));
    assert!(matches!(expired, ServerError::BadRequest(_)));
}

#[tokio::test]
async fn accept_invite_rejects_missing_invalid_expired_and_exhausted_invites() {
    let server_store = Arc::new(InMemoryServerStore::default());
    let state = state_with_store(server_store.clone());
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "accept_error_owner".to_owned(),
            email: "accept-error-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let first_guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "accept_error_first_guest".to_owned(),
            email: "accept-error-first-guest@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("first guest registration should succeed");
    let second_guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "accept_error_second_guest".to_owned(),
            email: "accept-error-second-guest@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("second guest registration should succeed");
    let server = create(
        &state,
        &owner_auth.access_token,
        CreateServerRequest {
            name: "Accept Errors".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
    let server_id = Uuid::parse_str(&server.server.id).expect("server id should be uuid");
    let owner_user_id = Uuid::parse_str(&owner_auth.user.id).expect("user id should be uuid");
    let expired_invite = server_store
        .insert_server_invite(
            &server_id,
            &owner_user_id,
            None,
            Some(chrono::Utc::now() - chrono::Duration::days(1)),
        )
        .await
        .expect("expired invite should be inserted");
    let limited_invite = create_invite(
        &state,
        &owner_auth.access_token,
        server.server.id,
        CreateServerInviteRequest {
            max_uses: Some(1),
            expires_in_days: None,
        },
    )
    .await
    .expect("limited invite should be created");

    accept_invite(
        &state,
        &first_guest_auth.access_token,
        limited_invite.code.clone(),
    )
    .await
    .expect("first use should succeed");

    let invalid = accept_invite(
        &state,
        &second_guest_auth.access_token,
        "not-a-uuid".to_owned(),
    )
    .await
    .expect_err("invalid invite code should fail");
    let missing = accept_invite(
        &state,
        &second_guest_auth.access_token,
        Uuid::new_v4().to_string(),
    )
    .await
    .expect_err("missing invite should fail");
    let expired = accept_invite(
        &state,
        &second_guest_auth.access_token,
        expired_invite.id.to_string(),
    )
    .await
    .expect_err("expired invite should fail");
    let exhausted = accept_invite(&state, &second_guest_auth.access_token, limited_invite.code)
        .await
        .expect_err("exhausted invite should fail");

    assert!(matches!(invalid, ServerError::BadRequest(_)));
    assert!(matches!(missing, ServerError::NotFound(_)));
    assert!(matches!(expired, ServerError::BadRequest(_)));
    assert!(matches!(exhausted, ServerError::BadRequest(_)));
}

#[tokio::test]
async fn member_can_join_again_after_soft_leave() {
    let server_store = Arc::new(InMemoryServerStore::default());
    let state = state_with_store(server_store.clone());
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "rejoin_owner".to_owned(),
            email: "rejoin-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "rejoin_guest".to_owned(),
            email: "rejoin-guest@example.com".to_owned(),
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
            name: "Rejoin".to_owned(),
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
    let server_id = Uuid::parse_str(&server.server.id).expect("server id should be uuid");
    let guest_user_id = Uuid::parse_str(&guest_auth.user.id).expect("user id should be uuid");

    accept_invite(&state, &guest_auth.access_token, invite.code.clone())
        .await
        .expect("first join should succeed");
    server_store
        .leave_server(&server_id, &guest_user_id)
        .await
        .expect("test member should soft leave");
    let second_join = accept_invite(&state, &guest_auth.access_token, invite.code)
        .await
        .expect("second join should succeed");
    let guest_members = server_store
        .members_for_tests()
        .expect("members should be readable")
        .into_iter()
        .filter(|member| member.server_id == server_id && member.user_id == guest_user_id)
        .collect::<Vec<_>>();
    let invite_uses = server_store
        .invite_uses_for_tests()
        .expect("invite uses should be readable");

    assert!(!second_join.already_member);
    assert_eq!(guest_members.len(), 2);
    assert_eq!(invite_uses.len(), 2);
}

#[tokio::test]
async fn non_owner_cannot_create_server_invite() {
    let state = state();
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "owner_user".to_owned(),
            email: "owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "guest_user".to_owned(),
            email: "guest@example.com".to_owned(),
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
            name: "Private".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");

    let error = create_invite(
        &state,
        &guest_auth.access_token,
        server.server.id,
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect_err("non-owner invite creation should fail");

    assert!(matches!(
        error,
        crate::features::servers::error::ServerError::NotFound(_)
    ));
}

#[tokio::test]
async fn create_invite_rejects_invalid_settings() {
    let state = state();
    let auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invalid_invite_owner".to_owned(),
            email: "invalid-invite-owner@example.com".to_owned(),
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
            name: "Validation".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");

    assert!(
        create_invite(
            &state,
            &auth.access_token,
            server.server.id.clone(),
            CreateServerInviteRequest {
                max_uses: Some(0),
                expires_in_days: None,
            },
        )
        .await
        .is_err()
    );
    assert!(
        create_invite(
            &state,
            &auth.access_token,
            server.server.id,
            CreateServerInviteRequest {
                max_uses: None,
                expires_in_days: Some(366),
            },
        )
        .await
        .is_err()
    );
}

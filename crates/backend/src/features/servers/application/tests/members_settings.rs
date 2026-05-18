use cheenhub_contracts::realtime::{KickServerMember, ListServerMembers};

use super::*;

#[tokio::test]
async fn owner_can_list_members_with_invite_used_to_join() {
    let state = state();
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "member_settings_owner".to_owned(),
            email: "member-settings-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "member_settings_guest".to_owned(),
            email: "member-settings-guest@example.com".to_owned(),
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
            name: "Member Settings".to_owned(),
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

    let response = list_server_members(
        &state,
        &owner_id,
        ListServerMembers {
            server_id: server.server.id,
        },
    )
    .await
    .expect("members should load");

    assert_eq!(response.members.len(), 2);
    let owner = response
        .members
        .iter()
        .find(|member| member.user_id == owner_auth.user.id)
        .expect("owner should be listed");
    assert!(owner.is_owner);
    assert!(owner.invite_code.is_none());
    let guest = response
        .members
        .iter()
        .find(|member| member.user_id == guest_auth.user.id)
        .expect("guest should be listed");
    assert_eq!(guest.nickname, guest_auth.user.nickname);
    assert_eq!(guest.invite_code.as_deref(), Some(invite.code.as_str()));
    assert!(guest.invite_used_at.is_some());
}

#[tokio::test]
async fn kicked_member_cannot_rejoin_until_exclusion_expires() {
    let state = state();
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "member_kick_owner".to_owned(),
            email: "member-kick-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "member_kick_guest".to_owned(),
            email: "member-kick-guest@example.com".to_owned(),
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
            name: "Member Kick".to_owned(),
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

    let kicked = kick_server_member(
        &state,
        &owner_id,
        KickServerMember {
            server_id: server.server.id.clone(),
            user_id: guest_auth.user.id.clone(),
            exclusion_duration_seconds: Some(3600),
        },
    )
    .await
    .expect("member should be kicked");
    let members = list_server_members(
        &state,
        &owner_id,
        ListServerMembers {
            server_id: server.server.id,
        },
    )
    .await
    .expect("members should load after kick");
    let rejoin = accept_invite(&state, &guest_auth.access_token, invite.code).await;

    assert_eq!(kicked.user_id, guest_auth.user.id);
    assert!(kicked.excluded_until.is_some());
    assert!(
        members
            .members
            .iter()
            .all(|member| member.user_id != guest_auth.user.id)
    );
    assert!(rejoin.is_err());
}

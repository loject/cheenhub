use super::*;

#[tokio::test]
async fn user_can_delete_own_invite_link() {
    let state = state();
    let auth = register(&state, "delete_owner").await;
    let server = create_server(&state, &auth.access_token, "Delete Hub").await;
    let first = create_invite(
        &state,
        &auth.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("invite creation should succeed");
    create_invite(
        &state,
        &auth.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("second invite creation should succeed");

    let response = delete_own_invite(
        &state,
        &auth.access_token,
        server.id.clone(),
        first.code.clone(),
    )
    .await
    .expect("own invite deletion should succeed");

    assert_eq!(response.code, first.code);
    assert_eq!(response.links.len(), 1);
    assert_ne!(response.links[0].code, first.code);
    assert_eq!(response.limit, None);

    // Строка приглашения сохраняется с отметкой удаления, чтобы не терять источник входа.
    let stored = server_store_invites(&state, &server).await;
    assert_eq!(stored.len(), 2);
    let deleted = stored
        .iter()
        .find(|invite| invite.id.to_string() == first.code)
        .expect("deleted invite should stay stored");
    let kept = stored
        .iter()
        .find(|invite| invite.id.to_string() != first.code)
        .expect("other invite should stay stored");
    assert!(deleted.deleted_at.is_some());
    assert!(kept.deleted_at.is_none());
}

#[tokio::test]
async fn deleted_invite_link_stops_working_but_keeps_join_attribution() {
    let state = state();
    let owner = register(&state, "attribution_owner").await;
    let member = register(&state, "attribution_member").await;
    let latecomer = register(&state, "attribution_latecomer").await;
    let server = create_server(&state, &owner.access_token, "Attribution Hub").await;
    let invite = create_invite(
        &state,
        &owner.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("invite creation should succeed");
    accept_invite(&state, &member.access_token, invite.code.clone())
        .await
        .expect("member should join through the invite");
    delete_own_invite(
        &state,
        &owner.access_token,
        server.id.clone(),
        invite.code.clone(),
    )
    .await
    .expect("own invite deletion should succeed");

    // Удалённая ссылка больше не пускает новых участников.
    let error = accept_invite(&state, &latecomer.access_token, invite.code.clone())
        .await
        .expect_err("deleted invite should not accept new members");
    assert!(matches!(error, ServerError::NotFound(_)));

    // Владелец по-прежнему видит ссылку и знает, кто по ней пришёл.
    let owner_id = Uuid::parse_str(&owner.user.id).expect("owner id should be uuid");
    let listed = list_server_invites(
        &state,
        &owner_id,
        ListServerInvites {
            server_id: server.id.clone(),
        },
    )
    .await
    .expect("invite listing should succeed");
    let stored = listed
        .invites
        .iter()
        .find(|stored| stored.code == invite.code)
        .expect("deleted invite should stay visible in server settings");

    assert!(stored.deleted_at.is_some());
    assert_eq!(stored.joined_members.len(), 1);
    assert_eq!(stored.joined_members[0].user_id, member.user.id);
}

#[tokio::test]
async fn user_cannot_delete_invite_link_created_by_someone_else() {
    let state = state();
    let owner = register(&state, "foreign_owner").await;
    let member = register(&state, "foreign_member").await;
    let server = create_server(&state, &owner.access_token, "Foreign Hub").await;
    let owner_invite = create_invite(
        &state,
        &owner.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("owner invite creation should succeed");
    accept_invite(&state, &member.access_token, owner_invite.code.clone())
        .await
        .expect("member should join through the invite");
    grant_create_invite_permission(&state, &owner, &server, &member.user.id).await;

    let error = delete_own_invite(
        &state,
        &member.access_token,
        server.id.clone(),
        owner_invite.code.clone(),
    )
    .await
    .expect_err("deleting a foreign invite link should be rejected");

    assert!(matches!(error, ServerError::NotFound(_)));
    let owner_links = list_own_invite_links(&state, &owner.access_token, server.id.clone())
        .await
        .expect("owner link listing should succeed");
    assert_eq!(owner_links.links.len(), 1);
}

#[tokio::test]
async fn own_invite_links_are_not_available_without_invite_permission() {
    let state = state();
    let owner = register(&state, "denied_owner").await;
    let guest = register(&state, "denied_guest").await;
    let server = create_server(&state, &owner.access_token, "Denied Hub").await;
    let owner_invite = create_invite(
        &state,
        &owner.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("owner invite creation should succeed");
    accept_invite(&state, &guest.access_token, owner_invite.code.clone())
        .await
        .expect("guest should join through the invite");
    remove_create_invite_permission(&state, &server).await;

    let error = list_own_invite_links(&state, &guest.access_token, server.id.clone())
        .await
        .expect_err("listing without invite permission should be rejected");

    assert!(matches!(error, ServerError::NotFound(_)));
}

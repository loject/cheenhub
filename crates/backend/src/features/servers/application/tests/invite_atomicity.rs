//! Тесты атомарного принятия приглашений сервера.

use super::*;

#[tokio::test]
async fn concurrent_last_invite_use_has_one_winner() {
    let server_store = Arc::new(InMemoryServerStore::default());
    let state = state_with_store(server_store.clone());
    let owner = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "atomic_invite_owner".to_owned(),
            email: "atomic-invite-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_terms: true,
            accepts_personal_data: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let first_guest = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "atomic_invite_first".to_owned(),
            email: "atomic-invite-first@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_terms: true,
            accepts_personal_data: true,
        },
    )
    .await
    .expect("first guest registration should succeed");
    let second_guest = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "atomic_invite_second".to_owned(),
            email: "atomic-invite-second@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_terms: true,
            accepts_personal_data: true,
        },
    )
    .await
    .expect("second guest registration should succeed");
    let server = create(
        &state,
        &owner.access_token,
        CreateServerRequest {
            name: "Atomic Invite".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
    let invite = create_invite(
        &state,
        &owner.access_token,
        server.server.id.clone(),
        CreateServerInviteRequest {
            max_uses: Some(1),
            expires_in_days: None,
        },
    )
    .await
    .expect("limited invite should be created");

    let (first, second) = tokio::join!(
        accept_invite(&state, &first_guest.access_token, invite.code.clone(),),
        accept_invite(&state, &second_guest.access_token, invite.code),
    );

    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let server_id = Uuid::parse_str(&server.server.id).expect("server id should parse");
    let guest_ids = [
        Uuid::parse_str(&first_guest.user.id).expect("first guest id should parse"),
        Uuid::parse_str(&second_guest.user.id).expect("second guest id should parse"),
    ];
    let accepted_guest_members = server_store
        .members_for_tests()
        .expect("members should be readable")
        .into_iter()
        .filter(|member| {
            member.server_id == server_id
                && guest_ids.contains(&member.user_id)
                && member.left_at.is_none()
        })
        .count();
    assert_eq!(accepted_guest_members, 1);
    assert_eq!(
        server_store
            .invite_uses_for_tests()
            .expect("invite uses should be readable")
            .len(),
        1
    );
}

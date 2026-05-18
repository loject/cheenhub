use std::io::Cursor;

use image::{ImageBuffer, ImageFormat, Rgba};

use super::*;

#[tokio::test]
async fn owner_can_update_server_name() {
    let state = state();
    let auth = registered_user(
        &state,
        "server_profile_owner",
        "server-profile-owner@example.com",
    )
    .await;
    let server = create(
        &state,
        &auth.access_token,
        CreateServerRequest {
            name: "Old Server".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");

    let updated = update(
        &state,
        &auth.access_token,
        server.server.id.clone(),
        UpdateServerRequest {
            name: "  New Server  ".to_owned(),
        },
    )
    .await
    .expect("server update should succeed");
    let listed = list(&state, &auth.access_token)
        .await
        .expect("server list should load");

    assert_eq!(updated.server.name, "New Server");
    assert_eq!(updated.server.avatar_url, None);
    assert_eq!(listed.servers[0], updated.server);
}

#[tokio::test]
async fn non_owner_cannot_update_server_name() {
    let state = state();
    let owner_auth = registered_user(
        &state,
        "server_profile_real_owner",
        "server-profile-real@example.com",
    )
    .await;
    let guest_auth = registered_user(
        &state,
        "server_profile_not_owner",
        "server-profile-not-owner@example.com",
    )
    .await;
    let server = create(
        &state,
        &owner_auth.access_token,
        CreateServerRequest {
            name: "Private Server".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");

    let error = update(
        &state,
        &guest_auth.access_token,
        server.server.id,
        UpdateServerRequest {
            name: "Taken".to_owned(),
        },
    )
    .await
    .expect_err("non-owner update should fail");

    assert!(matches!(error, ServerError::NotFound(_)));
}

#[tokio::test]
async fn owner_can_update_server_avatar() {
    let state = state();
    let auth = registered_user(
        &state,
        "server_avatar_owner",
        "server-avatar-owner@example.com",
    )
    .await;
    let server = create(
        &state,
        &auth.access_token,
        CreateServerRequest {
            name: "Avatar Server".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");

    let updated = update_avatar(
        &state,
        &auth.access_token,
        server.server.id.clone(),
        Bytes::from(png(64, 64)),
    )
    .await
    .expect("avatar update should succeed");

    let avatar_url = updated
        .server
        .avatar_url
        .expect("updated server should include avatar url");
    assert!(avatar_url.starts_with("http://localhost/api/images/"));
    assert_eq!(updated.server.name, "Avatar Server");
}

#[tokio::test]
async fn server_avatar_rejects_invalid_image() {
    let state = state();
    let auth = registered_user(&state, "server_bad_avatar", "server-bad-avatar@example.com").await;
    let server = create(
        &state,
        &auth.access_token,
        CreateServerRequest {
            name: "Avatar Server".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");

    let error = update_avatar(
        &state,
        &auth.access_token,
        server.server.id,
        Bytes::from_static(b"not-image"),
    )
    .await
    .expect_err("invalid image should fail");

    assert!(matches!(error, ServerError::BadRequest(_)));
}

async fn registered_user(
    state: &crate::state::AppState,
    nickname: &str,
    email: &str,
) -> cheenhub_contracts::rest::AuthResponse {
    auth_application::register(
        state,
        RegisterRequest {
            nickname: nickname.to_owned(),
            email: email.to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed")
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    let image = ImageBuffer::from_pixel(width, height, Rgba([30_u8, 120, 200, 255]));
    image
        .write_to(&mut bytes, ImageFormat::Png)
        .expect("test image should encode");
    bytes.into_inner()
}

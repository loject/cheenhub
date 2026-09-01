use cheenhub_contracts::rest::{CreateServerInviteRequest, RegisterRequest, ServerVoiceSettings};

use super::*;

async fn registered_user(
    state: &AppState,
    nickname: &str,
    email: &str,
) -> cheenhub_contracts::rest::AuthResponse {
    auth_application::register(
        state,
        RegisterRequest {
            nickname: nickname.to_owned(),
            email: email.to_owned(),
            password: "password123".to_owned(),
            accepts_terms: true,
            accepts_personal_data: true,
        },
    )
    .await
    .expect("registration should succeed")
}

#[tokio::test]
async fn owner_can_update_server_voice_settings() {
    let state = state();
    let auth = registered_user(
        &state,
        "voice_settings_owner",
        "voice-settings-owner@example.com",
    )
    .await;
    let server = create(
        &state,
        &auth.access_token,
        CreateServerRequest {
            name: "Voice Settings Hub".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");

    let updated = update_voice_settings(
        &state,
        &auth.access_token,
        server.server.id.clone(),
        ServerVoiceSettings {
            audio_bitrate_bps: 48_000,
        },
    )
    .await
    .expect("voice settings update should succeed");

    assert_eq!(updated.audio_bitrate_bps, 48_000);

    let loaded = get_voice_settings(&state, &auth.access_token, server.server.id)
        .await
        .expect("voice settings should load");
    assert_eq!(loaded.audio_bitrate_bps, 48_000);
}

#[tokio::test]
async fn non_owner_cannot_update_server_voice_settings() {
    let state = state();
    let owner_auth = registered_user(
        &state,
        "voice_settings_real_owner",
        "voice-settings-real-owner@example.com",
    )
    .await;
    let guest_auth = registered_user(
        &state,
        "voice_settings_guest",
        "voice-settings-guest@example.com",
    )
    .await;
    let server = create(
        &state,
        &owner_auth.access_token,
        CreateServerRequest {
            name: "Voice Settings Private".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");

    let error = update_voice_settings(
        &state,
        &guest_auth.access_token,
        server.server.id,
        ServerVoiceSettings {
            audio_bitrate_bps: 48_000,
        },
    )
    .await
    .expect_err("non-owner update should fail");

    assert!(matches!(error, ServerError::NotFound(_)));
}

#[tokio::test]
async fn voice_settings_rejects_out_of_range_bitrate() {
    let state = state();
    let auth = registered_user(
        &state,
        "voice_settings_range_owner",
        "voice-settings-range@example.com",
    )
    .await;
    let server = create(
        &state,
        &auth.access_token,
        CreateServerRequest {
            name: "Voice Settings Range".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");

    for bitrate in [8_000_u32, 128_000_u32] {
        let error = update_voice_settings(
            &state,
            &auth.access_token,
            server.server.id.clone(),
            ServerVoiceSettings {
                audio_bitrate_bps: bitrate,
            },
        )
        .await
        .expect_err("out-of-range bitrate should fail");

        assert!(matches!(error, ServerError::BadRequest(_)));
    }
}

#[tokio::test]
async fn member_can_read_server_voice_settings() {
    let state = state();
    let owner_auth = registered_user(
        &state,
        "voice_settings_member_owner",
        "voice-settings-member-owner@example.com",
    )
    .await;
    let member_auth = registered_user(
        &state,
        "voice_settings_member",
        "voice-settings-member@example.com",
    )
    .await;
    let server = create(
        &state,
        &owner_auth.access_token,
        CreateServerRequest {
            name: "Voice Settings Members".to_owned(),
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
    accept_invite(&state, &member_auth.access_token, invite.code)
        .await
        .expect("member should join");

    let loaded = get_voice_settings(&state, &member_auth.access_token, server.server.id)
        .await
        .expect("member voice settings should load");
    assert_eq!(
        loaded.audio_bitrate_bps,
        cheenhub_contracts::media::VOICE_AUDIO_BITRATE_BPS
    );
}

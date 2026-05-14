use std::io::Cursor;

use bytes::Bytes;
use cheenhub_contracts::rest::RegisterRequest;
use image::{GenericImageView, ImageBuffer, ImageFormat, Rgb, Rgba};
use tokio::time::{Duration, sleep};
use uuid::Uuid;

use crate::features::auth::application::{register, update_current_user_avatar};
use crate::features::images::application::public_image;
use crate::features::voice_chat::infrastructure::VoicePresence;

use super::state;

#[tokio::test]
async fn avatar_upload_returns_current_user_avatar_url() {
    let state = state();
    let auth = registered_user(&state).await;

    let updated = update_current_user_avatar(&state, &auth.access_token, Bytes::from(png(64, 64)))
        .await
        .expect("avatar upload should succeed");

    let avatar_url = updated.avatar_url.expect("avatar url should be returned");
    assert!(avatar_url.starts_with("http://localhost/api/images/"));
}

#[tokio::test]
async fn avatar_upload_rejects_invalid_image() {
    let state = state();
    let auth = registered_user(&state).await;

    let result =
        update_current_user_avatar(&state, &auth.access_token, Bytes::from_static(b"not-image"))
            .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn avatar_upload_rejects_oversized_image() {
    let state = state();
    let auth = registered_user(&state).await;
    let oversized = vec![0_u8; 8 * 1024 * 1024 + 1];

    let result =
        update_current_user_avatar(&state, &auth.access_token, Bytes::from(oversized)).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn avatar_upload_center_crops_and_stores_png_512() {
    let state = state();
    let auth = registered_user(&state).await;

    let updated =
        update_current_user_avatar(&state, &auth.access_token, Bytes::from(jpeg(800, 400)))
            .await
            .expect("avatar upload should succeed");
    let image_id = image_id_from_url(updated.avatar_url.as_deref().expect("avatar url"));
    let stored = public_image(&state, &image_id)
        .await
        .expect("stored avatar should be public");
    let data = stored.data.expect("database image bytes should be present");
    let decoded = image::load_from_memory(&data).expect("stored avatar should decode");

    assert_eq!(stored.content_type, "image/png");
    assert_eq!(decoded.dimensions(), (512, 512));
}

#[tokio::test]
async fn avatar_upload_updates_active_voice_presence() {
    let state = state();
    let auth = registered_user(&state).await;
    let user_id = Uuid::parse_str(&auth.user.id).expect("user id should be uuid");
    let server_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    state
        .voice_presence_store
        .join(VoicePresence {
            realtime_stream_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            server_id,
            room_id,
            user_id,
            nickname: auth.user.nickname.clone(),
            avatar_url: None,
            joined_at: chrono::Utc::now(),
        })
        .await;

    let updated = update_current_user_avatar(&state, &auth.access_token, Bytes::from(png(64, 64)))
        .await
        .expect("avatar upload should succeed");
    let participants = state
        .voice_presence_store
        .room_participants(&server_id, &room_id)
        .await;

    assert_eq!(participants[0].avatar_url, updated.avatar_url);
}

#[tokio::test]
async fn avatar_upload_waits_for_image_processing_queue() {
    let state = state();
    let auth = registered_user(&state).await;
    let permit = state
        .image_processing_queue
        .clone()
        .acquire_owned()
        .await
        .expect("queue should be open");
    let upload_state = state.clone();
    let access_token = auth.access_token.clone();
    let mut upload = tokio::spawn(async move {
        update_current_user_avatar(&upload_state, &access_token, Bytes::from(png(64, 64))).await
    });

    tokio::select! {
        result = &mut upload => panic!("avatar upload should wait for queue: {result:?}"),
        _ = sleep(Duration::from_millis(50)) => {}
    }

    drop(permit);
    let updated = upload
        .await
        .expect("queued upload task should finish")
        .expect("queued upload should succeed");
    assert!(updated.avatar_url.is_some());
}

async fn registered_user(state: &crate::state::AppState) -> cheenhub_contracts::rest::AuthResponse {
    register(
        state,
        RegisterRequest {
            nickname: "avatar_user".to_owned(),
            email: "avatar@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed")
}

fn png(width: u32, height: u32) -> Vec<u8> {
    image(width, height, ImageFormat::Png)
}

fn jpeg(width: u32, height: u32) -> Vec<u8> {
    image(width, height, ImageFormat::Jpeg)
}

fn image(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    if format == ImageFormat::Jpeg {
        let image = ImageBuffer::from_pixel(width, height, Rgb([30_u8, 120, 200]));
        image
            .write_to(&mut bytes, format)
            .expect("test image should encode");
    } else {
        let image = ImageBuffer::from_pixel(width, height, Rgba([30_u8, 120, 200, 255]));
        image
            .write_to(&mut bytes, format)
            .expect("test image should encode");
    }
    bytes.into_inner()
}

fn image_id_from_url(url: &str) -> Uuid {
    Uuid::parse_str(url.rsplit('/').next().expect("url should include image id"))
        .expect("image id should be uuid")
}

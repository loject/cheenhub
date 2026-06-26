use cheenhub_contracts::rest::ServerRoomKind;
use uuid::Uuid;

use super::super::{chat_image, upload_chat_image};
use super::{create_server_room, registered_user, state, tiny_png};

#[tokio::test]
async fn chat_image_upload_is_stored_and_served_through_proxy_flow() {
    let state = state();
    let auth = registered_user(&state, "image_owner", "image-owner@example.com").await;
    let user_id = Uuid::parse_str(&auth.user.id).expect("user id should be uuid");
    let (server_id, room_id) = create_server_room(
        &state,
        &user_id,
        "Images",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let bytes = tiny_png();

    let uploaded = upload_chat_image(
        &state,
        &user_id,
        server_id.clone(),
        room_id.clone(),
        Some("pixel.png".to_owned()),
        &bytes,
    )
    .await
    .expect("chat image should upload");

    assert_eq!(uploaded.server_id, server_id);
    assert_eq!(uploaded.room_id, room_id);
    assert_eq!(uploaded.content_type, "image/png");
    assert_eq!(uploaded.width, 1);
    assert_eq!(uploaded.height, 1);

    let (attachment, served) = chat_image(&state, &user_id, uploaded.id)
        .await
        .expect("chat image should serve");
    assert_eq!(attachment.content_type, "image/png");
    assert_eq!(served, bytes);
}

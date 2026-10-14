//! Регрессионные проверки операций закрепления сообщений.

use cheenhub_contracts::realtime::{
    DeleteMessage, ListPinnedMessages, PinMessage, ServerRoleKind, ServerRolePermission,
    UnpinMessage,
};
use cheenhub_contracts::rest::ServerRoomKind;
use chrono::Utc;
use uuid::Uuid;

use super::{create_server_room, registered_user, state};
use crate::features::servers::domain::ServerRole;
use crate::features::text_chat::application::{
    TextChatApplicationError, delete_message, list_pinned_messages, pin_message, unpin_message,
};
use crate::features::text_chat::domain::TextMessage;

async fn add_message(
    state: &crate::state::AppState,
    server_id: &str,
    room_id: &str,
    user_id: Uuid,
) -> String {
    let message_id = Uuid::new_v4();
    state
        .text_chat_store
        .insert_text_message(TextMessage {
            id: message_id,
            server_id: Uuid::parse_str(server_id).unwrap(),
            room_id: Uuid::parse_str(room_id).unwrap(),
            author_user_id: user_id,
            author_nickname: "Автор".to_owned(),
            body: "Важное сообщение".to_owned(),
            attachments: Vec::new(),
            created_at: Utc::now(),
            deleted_at: None,
            deleted_by_user_id: None,
        })
        .await
        .unwrap();
    message_id.to_string()
}

#[tokio::test]
async fn owner_can_pin_list_unpin_and_reject_duplicate() {
    let state = state();
    let owner = registered_user(&state, "pin_owner", "pin-owner@example.com").await;
    let owner_id = Uuid::parse_str(&owner.user.id).unwrap();
    let (server_id, room_id) = create_server_room(
        &state,
        &owner_id,
        "Сервер",
        "общая",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let message_id = add_message(&state, &server_id, &room_id, owner_id).await;

    let accepted = pin_message(
        &state,
        &owner_id,
        PinMessage {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            message_id: message_id.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(accepted.pinned.message.id, message_id);
    let duplicate = pin_message(
        &state,
        &owner_id,
        PinMessage {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            message_id: message_id.clone(),
        },
    )
    .await;
    assert!(matches!(
        duplicate,
        Err(TextChatApplicationError::BadRequest(_))
    ));

    let listed = list_pinned_messages(
        &state,
        &owner_id,
        ListPinnedMessages {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(listed.messages.len(), 1);
    assert_eq!(listed.messages[0].message.id, message_id);

    unpin_message(
        &state,
        &owner_id,
        UnpinMessage {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            message_id,
        },
    )
    .await
    .unwrap();
    let listed = list_pinned_messages(&state, &owner_id, ListPinnedMessages { server_id, room_id })
        .await
        .unwrap();
    assert!(listed.messages.is_empty());
}

#[tokio::test]
async fn outsider_cannot_view_or_modify_room_pins() {
    let state = state();
    let owner = registered_user(&state, "pin_owner2", "pin-owner2@example.com").await;
    let outsider = registered_user(&state, "pin_outsider", "pin-outsider@example.com").await;
    let owner_id = Uuid::parse_str(&owner.user.id).unwrap();
    let outsider_id = Uuid::parse_str(&outsider.user.id).unwrap();
    let (server_id, room_id) = create_server_room(
        &state,
        &owner_id,
        "Закрытый сервер",
        "общая",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let message_id = add_message(&state, &server_id, &room_id, owner_id).await;

    let list = list_pinned_messages(
        &state,
        &outsider_id,
        ListPinnedMessages {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
        },
    )
    .await;
    assert!(matches!(
        list,
        Err(TextChatApplicationError::Unauthorized(_))
    ));
    let pin = pin_message(
        &state,
        &outsider_id,
        PinMessage {
            server_id,
            room_id,
            message_id,
        },
    )
    .await;
    assert!(matches!(
        pin,
        Err(TextChatApplicationError::Unauthorized(_))
    ));
}

#[tokio::test]
async fn foreign_server_message_cannot_be_pinned_with_local_permissions() {
    let state = state();
    let owner = registered_user(&state, "pin_owner3", "pin-owner3@example.com").await;
    let owner_id = Uuid::parse_str(&owner.user.id).unwrap();
    let (server_id, room_id) = create_server_room(
        &state,
        &owner_id,
        "Первый сервер",
        "общая",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let (foreign_server_id, foreign_room_id) = create_server_room(
        &state,
        &owner_id,
        "Другой сервер",
        "общая",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let message_id = add_message(&state, &foreign_server_id, &foreign_room_id, owner_id).await;

    let result = pin_message(
        &state,
        &owner_id,
        PinMessage {
            server_id,
            room_id,
            message_id,
        },
    )
    .await;

    assert!(matches!(result, Err(TextChatApplicationError::NotFound(_))));
}

#[tokio::test]
async fn member_without_permission_can_read_but_cannot_modify_pins() {
    let state = state();
    let owner = registered_user(&state, "pin_role_owner", "pin-role-owner@example.com").await;
    let member = registered_user(&state, "pin_role_member", "pin-role-member@example.com").await;
    let owner_id = Uuid::parse_str(&owner.user.id).unwrap();
    let member_id = Uuid::parse_str(&member.user.id).unwrap();
    let (server_id, room_id) = create_server_room(
        &state,
        &owner_id,
        "Сервер",
        "общая",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let server_uuid = Uuid::parse_str(&server_id).unwrap();
    state
        .server_store
        .insert_server_member(&server_uuid, &member_id)
        .await
        .unwrap();
    let pinned_id = add_message(&state, &server_id, &room_id, owner_id).await;
    let other_id = add_message(&state, &server_id, &room_id, owner_id).await;
    pin_message(
        &state,
        &owner_id,
        PinMessage {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            message_id: pinned_id.clone(),
        },
    )
    .await
    .unwrap();

    let listed = list_pinned_messages(
        &state,
        &member_id,
        ListPinnedMessages {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(listed.messages.len(), 1);
    assert_eq!(listed.messages[0].message.id, pinned_id);

    let rejected_pin = pin_message(
        &state,
        &member_id,
        PinMessage {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            message_id: other_id,
        },
    )
    .await;
    assert!(matches!(
        rejected_pin,
        Err(TextChatApplicationError::Unauthorized(_))
    ));
    let rejected_unpin = unpin_message(
        &state,
        &member_id,
        UnpinMessage {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            message_id: pinned_id,
        },
    )
    .await;
    assert!(matches!(
        rejected_unpin,
        Err(TextChatApplicationError::Unauthorized(_))
    ));

    let listed = list_pinned_messages(&state, &owner_id, ListPinnedMessages { server_id, room_id })
        .await
        .unwrap();
    assert_eq!(listed.messages.len(), 1);
}

#[tokio::test]
async fn member_with_pin_permission_can_pin_and_unpin() {
    let state = state();
    let owner = registered_user(&state, "pin_grant_owner", "pin-grant-owner@example.com").await;
    let member = registered_user(&state, "pin_grant_member", "pin-grant-member@example.com").await;
    let owner_id = Uuid::parse_str(&owner.user.id).unwrap();
    let member_id = Uuid::parse_str(&member.user.id).unwrap();
    let (server_id, room_id) = create_server_room(
        &state,
        &owner_id,
        "Сервер",
        "общая",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let server_uuid = Uuid::parse_str(&server_id).unwrap();
    let role_id = Uuid::new_v4();
    state
        .server_store
        .replace_server_roles(
            &server_uuid,
            vec![ServerRole {
                id: role_id,
                server_id: server_uuid,
                name: "Модератор закреплений".to_owned(),
                color: "#22c55e".to_owned(),
                kind: ServerRoleKind::Custom,
                position: 0,
                permissions: vec![ServerRolePermission::PinMessages],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }],
        )
        .await
        .unwrap();
    state
        .server_store
        .insert_server_member(&server_uuid, &member_id)
        .await
        .unwrap();
    state
        .server_store
        .assign_server_member_role(&server_uuid, &member_id, &role_id, &owner_id)
        .await
        .unwrap();
    let message_id = add_message(&state, &server_id, &room_id, owner_id).await;

    let accepted = pin_message(
        &state,
        &member_id,
        PinMessage {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            message_id: message_id.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(accepted.pinned.pinned_by_user_id, member_id.to_string());

    unpin_message(
        &state,
        &member_id,
        UnpinMessage {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            message_id,
        },
    )
    .await
    .unwrap();
    let listed = list_pinned_messages(&state, &owner_id, ListPinnedMessages { server_id, room_id })
        .await
        .unwrap();
    assert!(listed.messages.is_empty());
}

#[tokio::test]
async fn pin_limit_rejects_fifty_first_message() {
    let state = state();
    let owner = registered_user(&state, "pin_limit_owner", "pin-limit-owner@example.com").await;
    let owner_id = Uuid::parse_str(&owner.user.id).unwrap();
    let (server_id, room_id) = create_server_room(
        &state,
        &owner_id,
        "Сервер",
        "общая",
        ServerRoomKind::TextAndVoice,
    )
    .await;

    for _ in 0..50 {
        let message_id = add_message(&state, &server_id, &room_id, owner_id).await;
        pin_message(
            &state,
            &owner_id,
            PinMessage {
                server_id: server_id.clone(),
                room_id: room_id.clone(),
                message_id,
            },
        )
        .await
        .unwrap();
    }
    let extra_id = add_message(&state, &server_id, &room_id, owner_id).await;

    let rejected = pin_message(
        &state,
        &owner_id,
        PinMessage {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            message_id: extra_id,
        },
    )
    .await;
    assert!(matches!(
        rejected,
        Err(TextChatApplicationError::BadRequest(_))
    ));

    let listed = list_pinned_messages(&state, &owner_id, ListPinnedMessages { server_id, room_id })
        .await
        .unwrap();
    assert_eq!(listed.limit, 50);
    assert_eq!(listed.messages.len(), 50);
}

#[tokio::test]
async fn deleting_pinned_message_clears_room_pins() {
    let state = state();
    let owner = registered_user(&state, "pin_owner4", "pin-owner4@example.com").await;
    let owner_id = Uuid::parse_str(&owner.user.id).unwrap();
    let (server_id, room_id) = create_server_room(
        &state,
        &owner_id,
        "Сервер",
        "общая",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let message_id = add_message(&state, &server_id, &room_id, owner_id).await;
    pin_message(
        &state,
        &owner_id,
        PinMessage {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            message_id: message_id.clone(),
        },
    )
    .await
    .unwrap();

    delete_message(
        &state,
        &owner_id,
        DeleteMessage {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            message_id,
        },
    )
    .await
    .unwrap();

    let listed = list_pinned_messages(&state, &owner_id, ListPinnedMessages { server_id, room_id })
        .await
        .unwrap();
    assert!(listed.messages.is_empty());
}

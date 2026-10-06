//! Адаптер отправки сообщений комнаты для общей формы.

use dioxus::prelude::*;
use futures_util::FutureExt;

use super::{messages::append_message, realtime, scroll::ScrollCommand};
use crate::features::message_composer::MessageOperations;
use crate::features::message_composer::pending_attachment::PendingImageAttachment;
use crate::features::realtime::RealtimeHandle;
use crate::features::typing::TypingNotifier;
use cheenhub_contracts::realtime::TextChatMessage;

pub(super) fn use_room_message_operations(
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
    mut messages: Signal<Vec<TextChatMessage>>,
    mut appearing_message_ids: Signal<Vec<String>>,
    mut pending_scroll: Signal<Option<ScrollCommand>>,
    typing: TypingNotifier,
) -> MessageOperations {
    let upload_realtime = realtime.clone();
    let upload_server_id = server_id.clone();
    let upload_room_id = room_id.clone();
    let upload = use_callback(move |attachment: PendingImageAttachment| {
        let realtime = upload_realtime.clone();
        let server_id = upload_server_id.clone();
        let room_id = upload_room_id.clone();
        async move {
            realtime::upload_chat_image(
                &realtime,
                server_id.clone(),
                room_id.clone(),
                attachment.file_name,
                attachment.bytes,
            )
            .await
            .map(|image| image.id)
            .map_err(|error| {
                warn!(%server_id, %room_id, %error, "room image upload failed");
                error.to_string()
            })
        }
        .boxed_local()
    });
    let send = use_callback(move |(body, attachment_id): (String, Option<String>)| {
        let realtime = realtime.clone();
        let server_id = server_id.clone();
        let room_id = room_id.clone();
        async move {
            let accepted = realtime::send_text_message(
                &realtime,
                server_id.clone(),
                room_id.clone(),
                body,
                attachment_id,
            )
            .await
            .map_err(|error| {
                warn!(%server_id, %room_id, %error, "room message send failed");
                error.to_string()
            })?;
            debug!(%server_id, %room_id, message_id = %accepted.message.id, "room message sent");
            if append_message(&mut messages, &mut appearing_message_ids, accepted.message) {
                pending_scroll.set(Some(ScrollCommand::Bottom));
            }
            Ok(())
        }
        .boxed_local()
    });
    MessageOperations {
        upload,
        send,
        typing,
    }
}

//! REST-адаптер отправки личных сообщений для общей формы.

use cheenhub_contracts::rest::DmMessageSummary;
use dioxus::prelude::*;
use futures_util::FutureExt;

use super::api;
use crate::features::message_composer::MessageOperations;
use crate::features::message_composer::pending_attachment::PendingImageAttachment;
use crate::features::typing::TypingNotifier;

pub(super) fn use_direct_message_operations(
    conversation_id: String,
    on_sent: EventHandler<DmMessageSummary>,
    typing: TypingNotifier,
) -> MessageOperations {
    let upload_conversation_id = conversation_id.clone();
    let upload = use_callback(move |image: PendingImageAttachment| {
        let conversation_id = upload_conversation_id.clone();
        async move {
            api::upload_dm_image(&conversation_id, image.bytes)
                .await
                .map(|image| image.id)
                .map_err(|error| {
                    warn!(%conversation_id, %error, "direct message image upload failed");
                    error
                })
        }
        .boxed_local()
    });
    let send = use_callback(move |(body, image_id): (String, Option<String>)| {
        let conversation_id = conversation_id.clone();
        async move {
            let message = api::send_dm_message(&conversation_id, body, image_id)
                .await
                .map_err(|error| {
                    warn!(%conversation_id, %error, "direct message send failed");
                    error
                })?;
            debug!(%conversation_id, message_id = %message.id, "direct message sent");
            on_sent.call(message);
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

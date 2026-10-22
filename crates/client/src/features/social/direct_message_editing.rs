//! Встроенное редактирование собственных личных сообщений.

use cheenhub_contracts::rest::DmMessageSummary;
use dioxus::prelude::*;

use crate::features::text_chat::InlineMessageEditState;

use super::api;

/// Подключает общий inline-редактор к REST API личных сообщений.
pub(super) fn use_direct_message_editing(
    conversation_id: String,
    current_user_id: String,
    mut messages: Signal<Vec<DmMessageSummary>>,
) -> (Callback<String>, Option<InlineMessageEditState>) {
    let mut editing_id = use_signal(|| None::<String>);
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let author_id = current_user_id.clone();

    let on_start = use_callback(move |message_id: String| {
        if saving() {
            return;
        }
        if messages()
            .iter()
            .any(|message| message.id == message_id && message.sender_user_id == author_id)
        {
            error.set(None);
            editing_id.set(Some(message_id));
        }
    });
    let on_cancel = use_callback(move |_| {
        if !saving() {
            editing_id.set(None);
            error.set(None);
        }
    });
    let on_save = use_callback(move |body: String| {
        if saving() {
            return;
        }
        let Some(message_id) = editing_id() else {
            return;
        };
        let Some(original) = messages()
            .into_iter()
            .find(|message| message.id == message_id && message.sender_user_id == current_user_id)
        else {
            return;
        };
        let body = body.trim().to_owned();
        if body.chars().count() > 4000
            || (body.is_empty() && original.image.is_none())
            || body == original.body
        {
            return;
        }

        let conversation_id = conversation_id.clone();
        saving.set(true);
        error.set(None);
        spawn(async move {
            match api::edit_dm_message(&conversation_id, &message_id, body).await {
                Ok(updated) => {
                    if let Some(existing) = messages
                        .write()
                        .iter_mut()
                        .find(|message| message.id == updated.id)
                    {
                        *existing = updated;
                    }
                    editing_id.set(None);
                    debug!(%conversation_id, %message_id, "direct message edit accepted");
                }
                Err(cause) => {
                    warn!(%conversation_id, %message_id, %cause, "direct message edit failed");
                    error.set(Some(
                        "Не удалось сохранить изменения. Проверь соединение и повтори попытку."
                            .to_owned(),
                    ));
                }
            }
            saving.set(false);
        });
    });

    let inline_edit = editing_id().map(|message_id| InlineMessageEditState {
        message_id,
        saving: saving(),
        error: error(),
        on_cancel,
        on_save,
    });
    (on_start, inline_edit)
}

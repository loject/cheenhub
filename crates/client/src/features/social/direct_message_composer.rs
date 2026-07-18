//! Форма отправки текста и изображений в выбранный личный диалог.

use cheenhub_contracts::rest::{DmConversationSummary, DmMessageSummary};
use dioxus::prelude::*;

use super::{api, clipboard};

const MAX_DM_IMAGE_BYTES: usize = 8 * 1024 * 1024;

/// Результат локальной команды формы личного сообщения.
#[derive(Debug, Clone)]
pub(super) enum DirectMessageComposerOutcome {
    /// Сообщение успешно создано сервером.
    MessageSent(DmMessageSummary),
}

/// Рендерит форму личного сообщения и самостоятельно владеет её временным состоянием.
#[component]
pub(super) fn DirectMessageComposer(
    conversation: DmConversationSummary,
    on_outcome: EventHandler<DirectMessageComposerOutcome>,
) -> Element {
    let mut draft = use_signal(String::new);
    let mut status = use_signal(String::new);
    let mut is_sending = use_signal(|| false);
    let mut is_uploading_image = use_signal(|| false);
    let mut is_reading_clipboard = use_signal(|| false);
    let busy = is_sending() || is_uploading_image() || is_reading_clipboard();
    let conversation_id = conversation.id.clone();
    let friend_nickname = conversation.friend_nickname.clone();

    let send_text_conversation_id = conversation_id.clone();
    let send_text = use_callback(move |_| {
        let body = draft().trim().to_owned();
        if body.is_empty() || is_sending() || is_uploading_image() {
            return;
        }
        let conversation_id = send_text_conversation_id.clone();
        is_sending.set(true);
        status.set(String::new());
        spawn(async move {
            match api::send_dm_message(&conversation_id, body, None).await {
                Ok(message) => {
                    debug!(conversation_id, message_id = %message.id, "sent direct message text");
                    draft.set(String::new());
                    on_outcome.call(DirectMessageComposerOutcome::MessageSent(message));
                }
                Err(error) => {
                    warn!(conversation_id, %error, "direct message text send failed");
                    status.set(error);
                }
            }
            is_sending.set(false);
        });
    });

    let select_image_conversation_id = conversation_id.clone();
    let select_image = use_callback(move |event: Event<FormData>| {
        if busy {
            return;
        }
        let Some(file) = event.files().into_iter().next() else {
            return;
        };
        if file.size() > MAX_DM_IMAGE_BYTES as u64 {
            status.set("Изображение слишком большое. Выберите файл до 8 МБ.".to_owned());
            return;
        }
        let conversation_id = select_image_conversation_id.clone();
        is_uploading_image.set(true);
        status.set(String::new());
        info!(conversation_id, file_name = %file.name(), file_size = file.size(), "reading selected direct message image");
        spawn(async move {
            let result = match file.read_bytes().await {
                Ok(bytes) => upload_and_send_image(&conversation_id, bytes.to_vec()).await,
                Err(_) => Err("Не удалось прочитать выбранное изображение.".to_owned()),
            };
            finish_image_send(result, &conversation_id, on_outcome, status);
            is_uploading_image.set(false);
        });
    });

    rsx! {
        div { class: "direct-message-composer-shell shrink-0 border-t border-zinc-800/80 bg-zinc-950/55 p-4 backdrop-blur-xl",
            div { class: crate::features::text_chat::CHAT_COMPOSER_CLASS,
                label {
                    class: "flex h-10 w-10 shrink-0 cursor-pointer items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-300 transition hover:bg-zinc-800 has-[:disabled]:cursor-not-allowed has-[:disabled]:opacity-45",
                    title: "Прикрепить изображение",
                    input {
                        class: "sr-only",
                        r#type: "file",
                        accept: "image/png,image/jpeg,image/gif,image/webp,image/*",
                        disabled: busy,
                        onchange: move |event| select_image.call(event),
                    }
                    if is_uploading_image() || is_reading_clipboard() {
                        span { class: "h-4 w-4 animate-spin rounded-full border-2 border-zinc-600 border-t-blue-300" }
                    } else {
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "m18.375 12.739-7.693 7.693a4.5 4.5 0 0 1-6.364-6.364l10.94-10.94a3 3 0 1 1 4.243 4.243L8.552 18.32a1.5 1.5 0 1 1-2.121-2.121l9.879-9.879" }
                        }
                    }
                }
                textarea {
                    rows: "1",
                    value: "{draft()}",
                    placeholder: "Сообщение для {friend_nickname}",
                    class: "max-h-28 min-h-10 flex-1 resize-none bg-transparent px-2 py-2 text-[13px] text-zinc-100 outline-none placeholder:text-zinc-600",
                    oninput: move |event| draft.set(event.value()),
                    onkeydown: move |event| {
                        if !busy
                            && event.key().to_string().eq_ignore_ascii_case("v")
                            && event.modifiers().ctrl()
                        {
                            let conversation_id = conversation_id.clone();
                            is_reading_clipboard.set(true);
                            spawn(async move {
                                match clipboard::read_image_png().await {
                                    Ok(Some(bytes)) => {
                                        info!(conversation_id, byte_size = bytes.len(), "read direct message image from clipboard");
                                        let result = upload_and_send_image(&conversation_id, bytes).await;
                                        finish_image_send(result, &conversation_id, on_outcome, status);
                                    }
                                    Ok(None) => {}
                                    Err(error) => {
                                        warn!(conversation_id, %error, "failed to read direct message image from clipboard");
                                        status.set(error);
                                    }
                                }
                                is_reading_clipboard.set(false);
                            });
                            return;
                        }
                        if event.key() == Key::Enter && !event.modifiers().shift() {
                            event.prevent_default();
                            send_text.call(());
                        }
                    },
                }
                button {
                    r#type: "button",
                    disabled: draft().trim().is_empty() || busy,
                    class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-blue-500 text-white transition hover:bg-blue-400 disabled:cursor-not-allowed disabled:opacity-45",
                    "aria-label": "Отправить сообщение",
                    onclick: move |_| send_text.call(()),
                    if is_sending() {
                        span { class: "h-4 w-4 animate-spin rounded-full border-2 border-blue-200/40 border-t-white" }
                    } else {
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 12 3.269 3.126A59.77 59.77 0 0 1 21.485 12 59.768 59.768 0 0 1 3.27 20.876L6 12Zm0 0h7.5" }
                        }
                    }
                }
            }
            if !status().is_empty() {
                p { class: "mx-auto mt-2 w-full max-w-5xl px-2 text-[11px] leading-4 text-red-200", "{status()}" }
            }
        }
    }
}

async fn upload_and_send_image(
    conversation_id: &str,
    bytes: Vec<u8>,
) -> Result<DmMessageSummary, String> {
    if bytes.len() > MAX_DM_IMAGE_BYTES {
        return Err("Изображение слишком большое. Максимум — 8 МБ.".to_owned());
    }
    let image = api::upload_dm_image(conversation_id, bytes).await?;
    api::send_dm_message(conversation_id, String::new(), Some(image.id)).await
}

fn finish_image_send(
    result: Result<DmMessageSummary, String>,
    conversation_id: &str,
    on_outcome: EventHandler<DirectMessageComposerOutcome>,
    mut status: Signal<String>,
) {
    match result {
        Ok(message) => {
            debug!(conversation_id, message_id = %message.id, "sent direct message image");
            on_outcome.call(DirectMessageComposerOutcome::MessageSent(message));
        }
        Err(error) => {
            warn!(conversation_id, %error, "direct message image send failed");
            status.set(error);
        }
    }
}

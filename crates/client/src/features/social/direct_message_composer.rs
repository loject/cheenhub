//! Форма отправки текста и изображений в выбранный личный диалог.

use std::{cell::Cell, rc::Rc};

use cheenhub_contracts::rest::{DmConversationSummary, DmMessageSummary};
use dioxus::prelude::*;

use crate::features::image_picker::{ImagePickerButton, ImagePickerOutcome, PickedImage};

use super::direct_message_attachment_preview::DirectMessageAttachmentPreview;
use super::direct_message_pending_image::{
    PendingDirectMessageImage, pending_direct_message_image,
};
use super::{api, clipboard};

const MAX_DM_IMAGE_BYTES: usize = 8 * 1024 * 1024;
const DIRECT_MESSAGE_COMPOSER_GROUP_CLASS: &str = "mx-auto min-w-0 w-full max-w-5xl space-y-2";
const DIRECT_MESSAGE_COMPOSER_CLASS: &str = concat!(
    "direct-message-input-wrap flex min-w-0 w-full items-end gap-2 rounded-[20px] ",
    "border border-zinc-800 bg-[rgba(39,39,42,.8)] p-2 ",
    "shadow-[0_0_0_1px_rgba(255,255,255,0.02)]"
);

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
    let mut pending_attachment = use_signal(|| None::<PendingDirectMessageImage>);
    let mut status = use_signal(String::new);
    let mut is_sending = use_signal(|| false);
    let mut is_selecting_image = use_signal(|| false);
    let mut is_reading_clipboard = use_signal(|| false);
    let mut composer_input_element = use_signal(|| None::<Rc<MountedData>>);
    let mut refocus_requested = use_signal(|| false);
    let component_current = Rc::new(Cell::new(true));
    use_drop({
        let component_current = component_current.clone();
        move || component_current.set(false)
    });
    let busy = is_sending() || is_selecting_image() || is_reading_clipboard();
    let can_send = !busy && (!draft().trim().is_empty() || pending_attachment().is_some());
    let conversation_id = conversation.id.clone();
    let friend_nickname = conversation.friend_nickname.clone();

    let send_conversation_id = conversation_id.clone();
    let send_component_current = component_current.clone();
    let send_message = use_callback(move |_| {
        let body = draft().trim().to_owned();
        let attachment = pending_attachment();
        if is_sending()
            || is_selecting_image()
            || is_reading_clipboard()
            || (body.is_empty() && attachment.is_none())
        {
            return;
        }
        let conversation_id = send_conversation_id.clone();
        let component_current = send_component_current.clone();
        is_sending.set(true);
        refocus_requested.set(true);
        status.set(String::new());
        spawn(async move {
            let image_id = match attachment {
                Some(attachment) => match attachment.uploaded_id {
                    Some(image_id) => Some(image_id),
                    None => match api::upload_dm_image(&conversation_id, attachment.bytes).await {
                        Ok(image) => {
                            info!(conversation_id, image_id = %image.id, "uploaded pending direct message image");
                            if let Some(pending) = pending_attachment.write().as_mut() {
                                pending.uploaded_id = Some(image.id.clone());
                            }
                            Some(image.id)
                        }
                        Err(error) => {
                            warn!(conversation_id, %error, "direct message image upload failed");
                            status.set(error);
                            is_sending.set(false);
                            restore_composer_input_focus(
                                composer_input_element,
                                refocus_requested,
                                component_current.clone(),
                            );
                            return;
                        }
                    },
                },
                None => None,
            };
            match api::send_dm_message(&conversation_id, body, image_id).await {
                Ok(message) => {
                    debug!(conversation_id, message_id = %message.id, "sent direct message");
                    draft.set(String::new());
                    pending_attachment.set(None);
                    on_outcome.call(DirectMessageComposerOutcome::MessageSent(message));
                }
                Err(error) => {
                    warn!(conversation_id, %error, "direct message send failed");
                    status.set(error);
                }
            }
            is_sending.set(false);
            restore_composer_input_focus(
                composer_input_element,
                refocus_requested,
                component_current,
            );
        });
    });

    let add_image_conversation_id = conversation_id.clone();
    let add_pending_image = use_callback(
        move |result: Result<PendingDirectMessageImage, String>| {
            if is_sending() || pending_attachment().is_some() {
                return;
            }
            match result {
                Ok(attachment) => {
                    info!(
                        conversation_id = add_image_conversation_id,
                        byte_size = attachment.byte_size,
                        has_file_name = attachment.file_name.is_some(),
                        "added pending direct message image"
                    );
                    status.set(String::new());
                    pending_attachment.set(Some(attachment));
                }
                Err(error) => {
                    warn!(conversation_id = add_image_conversation_id, %error, "direct message image rejected before upload");
                    status.set(error);
                }
            }
        },
    );
    let select_pending_image = use_callback(move |outcome: ImagePickerOutcome| {
        let result = match outcome {
            ImagePickerOutcome::Selected(PickedImage { file_name, bytes }) => {
                pending_direct_message_image(file_name, bytes, MAX_DM_IMAGE_BYTES)
            }
            ImagePickerOutcome::Failed(error) => Err(error),
        };
        add_pending_image.call(result);
    });
    let clipboard_outcome =
        use_callback(move |result: Result<PendingDirectMessageImage, String>| {
            is_reading_clipboard.set(false);
            add_pending_image.call(result);
        });
    let clipboard_started = use_callback(move |_| is_reading_clipboard.set(true));
    let remove_conversation_id = conversation_id.clone();

    rsx! {
        div { class: "direct-message-composer-shell min-w-0 shrink-0 border-t border-zinc-800/80 bg-zinc-950/55 p-4 backdrop-blur-xl",
            div { class: DIRECT_MESSAGE_COMPOSER_GROUP_CLASS,
                if is_reading_clipboard() {
                    div { class: "flex items-center gap-2 px-2 text-[11px] text-zinc-400", role: "status", "aria-live": "polite",
                        span { class: "size-3 animate-spin rounded-full border-2 border-zinc-600 border-t-blue-300", "aria-hidden": "true" }
                        "Получаем изображение из буфера обмена…"
                    }
                }
                if let Some(attachment) = pending_attachment() {
                    DirectMessageAttachmentPreview {
                        attachment,
                        busy: is_sending(),
                        on_remove: move |_| {
                            if !is_sending() {
                                info!(conversation_id = remove_conversation_id, "removed pending direct message image");
                                pending_attachment.set(None);
                                status.set(String::new());
                            }
                        }
                    }
                }
                div { class: DIRECT_MESSAGE_COMPOSER_CLASS,
                ImagePickerButton {
                    disabled: busy || pending_attachment().is_some(),
                    busy: is_selecting_image() || is_reading_clipboard(),
                    max_bytes: MAX_DM_IMAGE_BYTES,
                    on_outcome: move |outcome| select_pending_image.call(outcome),
                    on_active_change: move |active| is_selecting_image.set(active),
                }
                textarea {
                    rows: "1",
                    value: "{draft()}",
                    readonly: is_sending(),
                    placeholder: "Сообщение для {friend_nickname}",
                    class: "max-h-28 min-h-10 min-w-0 flex-1 resize-none bg-transparent px-2 py-2 text-[13px] text-zinc-100 outline-none placeholder:text-zinc-600",
                    onmounted: move |event| composer_input_element.set(Some(event.data.clone())),
                    oninput: move |event| draft.set(event.value()),
                    onblur: move |_| refocus_requested.set(false),
                    onpaste: move |event| {
                        if !is_sending()
                            && !is_selecting_image()
                            && !is_reading_clipboard()
                            && pending_attachment().is_none()
                            && clipboard::read_pasted_image(event, clipboard_outcome)
                        {
                            clipboard_started.call(());
                        }
                    },
                    onkeydown: move |event| {
                        if clipboard::supports_keydown_image_paste()
                            && !busy
                            && event.key().to_string().eq_ignore_ascii_case("v")
                            && event.modifiers().ctrl()
                        {
                            let conversation_id = conversation_id.clone();
                            is_reading_clipboard.set(true);
                            spawn(async move {
                                match clipboard::read_image_png().await {
                                    Ok(Some(bytes)) => clipboard_outcome.call(pending_direct_message_image(None, bytes, MAX_DM_IMAGE_BYTES)),
                                    Ok(None) => is_reading_clipboard.set(false),
                                    Err(error) => {
                                        warn!(conversation_id, %error, "failed to read direct message image from clipboard");
                                        status.set(error);
                                        is_reading_clipboard.set(false);
                                    }
                                }
                            });
                        }
                        if event.key() == Key::Enter && !event.modifiers().shift() {
                            event.prevent_default();
                            send_message.call(());
                        }
                    },
                }
                button {
                    r#type: "button",
                    disabled: !can_send,
                    class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-blue-500 text-white transition hover:bg-blue-400 disabled:cursor-not-allowed disabled:opacity-45",
                    "aria-label": "Отправить сообщение",
                    onpointerdown: move |event| {
                        event.prevent_default();
                        send_message.call(());
                    },
                    onclick: move |_| send_message.call(()),
                    if is_sending() {
                        span { class: "h-4 w-4 animate-spin rounded-full border-2 border-blue-200/40 border-t-white" }
                    } else {
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 12 3.269 3.126A59.77 59.77 0 0 1 21.485 12 59.768 59.768 0 0 1 3.27 20.876L6 12Zm0 0h7.5" } }
                    }
                }
                }
                if !status().is_empty() {
                    p { class: "px-2 text-[11px] leading-4 text-red-200", "aria-live": "polite", "{status()}" }
                }
            }
        }
    }
}

fn restore_composer_input_focus(
    input_element: Signal<Option<Rc<MountedData>>>,
    refocus_requested: Signal<bool>,
    component_current: Rc<Cell<bool>>,
) {
    if !should_refocus(component_current.get(), refocus_requested()) {
        return;
    }

    let Some(element) = input_element.cloned() else {
        return;
    };

    spawn(async move {
        if !should_refocus(component_current.get(), refocus_requested()) {
            return;
        }

        if let Err(error) = element.set_focus(true).await {
            debug!(?error, "failed to restore direct message input focus");
        }
    });
}

fn should_refocus(component_current: bool, refocus_requested: bool) -> bool {
    component_current && refocus_requested
}

#[cfg(test)]
mod tests {
    use super::should_refocus;

    #[test]
    fn refocus_requires_an_active_component_and_submit_intent() {
        assert!(should_refocus(true, true));
        assert!(!should_refocus(false, true));
        assert!(!should_refocus(true, false));
    }
}

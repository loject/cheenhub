//! Единый компонент ввода текста, вложений и эмодзи.

use dioxus::dioxus_core::current_scope_id;
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};

use super::attachment_preview::MessageAttachmentPreview;
use super::emoji_picker::EmojiPicker;
use super::pending_attachment::{
    PendingImageAttachment, can_send_message, pending_image_attachment,
};
use super::sending::submit_message;
use super::{MAX_IMAGE_BYTES, MessageComposeState, MessageOperations, clipboard};
use crate::features::image_picker::{ImagePickerButton, ImagePickerOutcome, PickedImage};
use crate::features::typing::{TypingIntent, TypingNotifier};

/// Рендерит одну форму сообщения; состояние принадлежит keyed-диалогу владельца.
#[component]
pub(crate) fn MessageComposer(
    state: MessageComposeState,
    operations: MessageOperations,
    placeholder: String,
    #[props(default)] compact: bool,
    #[props(default = true)] active: bool,
) -> Element {
    let mut state = state;
    let mut input_element = use_signal(|| None::<Rc<MountedData>>);
    let mut refocus_requested = use_signal(|| false);
    let picker_id = use_hook(|| format!("message-emojis-{}", current_scope_id().0));
    let mut emoji_button = use_signal(|| None::<Rc<MountedData>>);
    let mut picker_style = use_signal(|| {
        String::from("top: max(12px, calc(100dvh - 396px)); left: max(12px, calc(100dvw - 344px));")
    });
    let component_current = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let component_current = component_current.clone();
        let typing = operations.typing;
        move || {
            component_current.set(false);
            // Набор снимается при размонтировании формы: переключение комнаты или
            // диалога не должно оставлять у собеседников «зависший» индикатор.
            typing.notify(TypingIntent::Stopped);
            for mut active in [state.is_selecting_image, state.is_reading_clipboard] {
                if let Ok(mut value) = active.try_write() {
                    *value = false;
                }
            }
        }
    });
    let busy =
        (state.is_sending)() || (state.is_selecting_image)() || (state.is_reading_clipboard)();
    let can_send = can_send_message(
        &(state.draft)(),
        (state.pending_attachment)().is_some(),
        busy,
    );
    let shell_class = if compact {
        "message-composer-shell shrink-0 bg-[#08090b] px-3 pb-3 pt-2"
    } else {
        "message-composer-shell min-w-0 shrink-0 bg-[#08090b] px-5 pb-5 pt-2 lg:px-8"
    };
    let complete_current = component_current.clone();
    let on_complete = use_callback(move |_| {
        restore_input_focus(input_element, refocus_requested(), complete_current.clone());
    });
    let submit = use_callback(move |_| {
        operations.typing.notify(TypingIntent::Stopped);
        if can_send_message(
            &(state.draft)(),
            (state.pending_attachment)().is_some(),
            (state.is_sending)() || (state.is_selecting_image)() || (state.is_reading_clipboard)(),
        ) {
            refocus_requested.set(true);
            submit_message(state, operations, on_complete);
        }
    });
    let accept_image = use_callback(move |result: Result<PendingImageAttachment, String>| {
        state.is_reading_clipboard.set(false);
        if (state.is_sending)() || (state.pending_attachment)().is_some() {
            return;
        }
        match result {
            Ok(image) => {
                info!(
                    byte_size = image.byte_size,
                    "message composer image selected"
                );
                state.pending_attachment.set(Some(image));
                state.status.set(String::new());
            }
            Err(error) => {
                warn!(%error, "message composer image rejected");
                state.status.set(error);
            }
        }
    });
    let emoji_current = component_current.clone();
    rsx! {
        div { class: shell_class,
            div { class: "relative mx-auto min-w-0 w-full max-w-5xl space-y-2",
                if (state.is_reading_clipboard)() {
                    div { class: "flex items-center gap-2 px-2 text-[11px] text-zinc-400", role: "status", "aria-live": "polite",
                        span { class: "size-3 animate-spin rounded-full border-2 border-zinc-600 border-t-blue-300", "aria-hidden": "true" }
                        "Получаем изображение из буфера обмена…"
                    }
                }
                if let Some(attachment) = (state.pending_attachment)() {
                    MessageAttachmentPreview { attachment, busy: (state.is_sending)(), on_remove: move |_| {
                        if !(state.is_sending)() {
                            debug!("message composer image removed");
                            state.pending_attachment.set(None);
                            state.status.set(String::new());
                        }
                    } }
                }
                if active {
                EmojiPicker {
                    id: picker_id.clone(),
                    position_style: picker_style(),
                    disabled: busy,
                    on_select: move |emoji: String| {
                        if !(state.is_sending)() {
                            state.draft.write().push_str(&emoji);
                            debug!("message composer emoji inserted");
                            restore_input_focus(input_element, true, emoji_current.clone());
                        }
                    },
                    on_close: move |_| on_complete.call(()),
                }
                }
                div { class: "message-input-wrap flex min-w-0 w-full items-end gap-2 rounded-[20px] bg-[#181a20]/95 p-2 shadow-[0_0_0_1px_rgba(255,255,255,0.07),0_18px_50px_rgba(0,0,0,0.32)]",
                    ImagePickerButton {
                        disabled: busy || (state.pending_attachment)().is_some(),
                        busy: (state.is_selecting_image)() || (state.is_reading_clipboard)(),
                        max_bytes: MAX_IMAGE_BYTES,
                        on_outcome: move |outcome| accept_image.call(match outcome {
                            ImagePickerOutcome::Selected(PickedImage { file_name, bytes }) => pending_image_attachment(file_name, bytes, MAX_IMAGE_BYTES),
                            ImagePickerOutcome::Failed(error) => Err(error),
                        }),
                        on_active_change: move |active| state.is_selecting_image.set(active),
                    }
                    textarea {
                        rows: "1",
                        value: "{(state.draft)()}",
                        readonly: (state.is_sending)(),
                        placeholder,
                        style: "field-sizing: content;",
                        class: "max-h-80 min-h-10 min-w-0 flex-1 resize-none overflow-y-auto bg-transparent px-2 py-2 text-[13px] leading-5 text-zinc-100 outline-none placeholder:text-zinc-600",
                        onmounted: move |event| input_element.set(Some(event.data.clone())),
                        oninput: move |event| {
                            let value = event.value();
                            notify_typing(&operations.typing, &value);
                            state.draft.set(value);
                        },
                        onblur: move |_| refocus_requested.set(false),
                        onpaste: move |event| {
                            if !(state.is_sending)() && !(state.is_selecting_image)()
                                && !(state.is_reading_clipboard)() && (state.pending_attachment)().is_none()
                                && clipboard::read_pasted_image(event, accept_image) {
                                state.is_reading_clipboard.set(true);
                            }
                        },
                        onkeydown: move |event| {
                            if clipboard::supports_keydown_image_paste() && !busy
                                && (state.pending_attachment)().is_none()
                                && event.key().to_string().eq_ignore_ascii_case("v")
                                && (event.modifiers().ctrl() || event.modifiers().meta()) {
                                state.is_reading_clipboard.set(true);
                                spawn(async move {
                                    match clipboard::read_image_png().await {
                                        Ok(Some(bytes)) => accept_image.call(pending_image_attachment(None, bytes, MAX_IMAGE_BYTES)),
                                        Ok(None) => state.is_reading_clipboard.set(false),
                                        Err(error) => accept_image.call(Err(error)),
                                    }
                                });
                            }
                            if event.key() == Key::Enter && !event.modifiers().shift() && !event.is_composing() {
                                event.prevent_default();
                                submit.call(());
                            }
                        },
                    }
                    button {
                        r#type: "button", disabled: busy,
                        class: "message-emoji-button flex h-10 w-10 shrink-0 items-center justify-center rounded-xl text-zinc-400 transition-colors hover:bg-white/5 hover:text-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300 disabled:opacity-45",
                        "aria-label": "Выбрать эмодзи", title: "Эмодзи",
                        popovertarget: picker_id.clone(),
                        onmounted: move |event| emoji_button.set(Some(event.data.clone())),
                        onclick: move |_| {
                            if let Some(element) = emoji_button.cloned() {
                                spawn(async move {
                                    if let Ok(rect) = element.get_client_rect().await {
                                        let top = rect.origin.y - 384.0;
                                        let left = rect.origin.x + rect.size.width - 320.0;
                                        picker_style.set(format!("top: clamp(12px, {top}px, max(12px, calc(100dvh - 396px))); left: clamp(12px, {left}px, max(12px, calc(100dvw - 332px)));"));
                                    }
                                });
                            }
                        },
                        svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                            circle { cx: "12", cy: "12", r: "9" }
                            path { stroke_linecap: "round", d: "M8 14a4 4 0 0 0 8 0" }
                            circle { cx: "9", cy: "9", r: "1", fill: "currentColor", stroke: "none" }
                            circle { cx: "15", cy: "9", r: "1", fill: "currentColor", stroke: "none" }
                        }
                    }
                    button {
                        r#type: "button", disabled: !can_send,
                        class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-blue-500 text-white shadow-[0_0_0_1px_rgba(96,165,250,0.28),0_6px_18px_rgba(37,99,235,0.2)] transition-[background-color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-blue-400 active:scale-[0.96] disabled:cursor-not-allowed disabled:opacity-45 disabled:hover:translate-y-0 disabled:active:scale-100",
                        "aria-label": "Отправить сообщение",
                        onpointerdown: move |event| event.prevent_default(),
                        onclick: move |_| submit.call(()),
                        if (state.is_sending)() {
                            span { class: "size-4 animate-spin rounded-full border-2 border-blue-200/40 border-t-white", role: "status", "aria-label": "Отправляем сообщение" }
                        } else {
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 12 3.269 3.126A59.77 59.77 0 0 1 21.485 12 59.768 59.768 0 0 1 3.27 20.876L6 12Zm0 0h7.5" }
                            }
                        }
                    }
                }
                if !(state.status)().is_empty() {
                    p { class: "px-2 text-[11px] leading-4 text-red-200", "aria-live": "polite", "{(state.status)()}" }
                }
            }
        }
    }
}

/// Сообщает чату о начале или завершении набора по содержимому черновика.
///
/// Пустой черновик означает, что пользователь удалил текст и больше не печатает,
/// поэтому индикатор у собеседников должен погаснуть сразу, а не по таймауту.
fn notify_typing(typing: &TypingNotifier, draft: &str) {
    let intent = if draft.trim().is_empty() {
        TypingIntent::Stopped
    } else {
        TypingIntent::Started
    };
    typing.notify(intent);
}

fn restore_input_focus(
    input_element: Signal<Option<Rc<MountedData>>>,
    requested: bool,
    current: Rc<Cell<bool>>,
) {
    if !requested || !current.get() {
        return;
    }
    if let Some(element) = input_element.cloned() {
        spawn(async move {
            if current.get()
                && let Err(error) = element.set_focus(true).await
            {
                debug!(?error, "message composer focus restore failed");
            }
        });
    }
}

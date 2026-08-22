//! Компонент панели текстового чата комнаты.

use std::time::Duration;
use std::{cell::Cell, rc::Rc};

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::app::components::app_shell::ActiveRoom;
use crate::features::app::server_permissions::ServerPermissionsContext;
use crate::features::image_picker::{ImagePickerButton, ImagePickerOutcome, PickedImage};
use crate::features::realtime::RealtimeHandle;
use crate::features::runtime::sleep_duration;

use super::clipboard;
use super::compose::{ComposeState, send_current_message};
use super::compose_actions::add_pending_image;
use super::history::{
    HistoryState, HistoryTarget, load_initial_history, load_initial_history_when_connected,
    load_older_history,
};
use super::messages::{append_message, group_consecutive_messages, remove_message};
use super::pending_attachment::{
    PendingImageAttachment, can_send_message, pending_image_attachment,
};
use super::realtime::{self, TextChatEvent};
use super::scroll::{ScrollCommand, apply_scroll_command, update_scroll_state};
use super::{
    CHAT_COMPOSER_CLASS, CHAT_COMPOSER_GROUP_CLASS, CHAT_CONTENT_CLASS, ChatAttachmentPreview,
    ChatMessageDateDivider, ChatMessageGroup, RoomComposeState, friendly_message_date,
    message_day_key,
};

const MAX_CHAT_IMAGE_BYTES: usize = 10 * 1024 * 1024;

/// Рендерит панель realtime-текстового чата для одной комнаты.
#[component]
pub(crate) fn ChatRoomPanel(server_id: String, room: ActiveRoom, compact: bool) -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let permissions = use_context::<ServerPermissionsContext>();
    let room_compose_state = use_context::<RoomComposeState>();
    let mut messages = use_signal(Vec::<TextChatMessage>::new);
    let mut appearing_message_ids = use_signal(Vec::<String>::new);
    let mut removing_message_ids = use_signal(Vec::<String>::new);
    let mut draft = room_compose_state.draft;
    let mut status = room_compose_state.status;
    let is_sending = room_compose_state.is_sending;
    let mut is_selecting_image = room_compose_state.is_selecting_image;
    let mut is_reading_clipboard = room_compose_state.is_reading_clipboard;
    let mut pending_attachment = room_compose_state.pending_attachment;
    let initial_loading = use_signal(|| true);
    let older_loading = use_signal(|| false);
    let history_error = use_signal(|| None::<String>);
    let older_error = use_signal(|| None::<String>);
    let has_more = use_signal(|| false);
    let is_near_bottom = use_signal(|| true);
    let mut list_element = use_signal(|| None::<Rc<MountedData>>);
    let mut compose_input_element = use_signal(|| None::<Rc<MountedData>>);
    let mut refocus_requested = use_signal(|| false);
    let component_current = Rc::new(Cell::new(true));
    use_drop({
        let component_current = component_current.clone();
        move || component_current.set(false)
    });
    let mut pending_scroll = use_signal(|| None::<ScrollCommand>);
    let event_room_id = room.id.clone();
    let history_server_id = server_id.clone();
    let history_room_id = room.id.clone();
    let older_server_id = server_id.clone();
    let older_room_id = room.id.clone();
    let send_server_id = server_id.clone();
    let send_room_id = room.id.clone();
    let delete_server_id = server_id.clone();
    let delete_room_id = room.id.clone();
    let delete_realtime = realtime.clone();
    let history_realtime = realtime.clone();
    let event_realtime = realtime.clone();
    let older_realtime = realtime.clone();
    let send_realtime = realtime.clone();
    let history_target = HistoryTarget {
        realtime: history_realtime,
        server_id: history_server_id,
        room_id: history_room_id,
    };
    let older_target = HistoryTarget {
        realtime: older_realtime,
        server_id: older_server_id,
        room_id: older_room_id,
    };
    let history_state = HistoryState {
        messages,
        appearing_message_ids,
        has_more,
        initial_loading,
        history_error,
        older_loading,
        older_error,
        list_element,
        pending_scroll,
    };
    let compose_state = ComposeState {
        draft,
        messages,
        appearing_message_ids,
        status,
        is_sending,
        pending_attachment,
        pending_scroll,
    };
    let placeholder_prefix = if compact { "&" } else { "#" };
    let list_class = if compact {
        "min-h-0 flex-1 overflow-y-auto p-4 pt-2"
    } else {
        "min-h-0 flex-1 overflow-y-auto p-5 lg:p-6"
    };
    let inner_class = if compact {
        "space-y-4"
    } else {
        CHAT_CONTENT_CLASS
    };
    let input_outer_class = if compact {
        "shrink-0 border-t border-zinc-800/80 bg-zinc-950/35 p-3"
    } else {
        "shrink-0 border-t border-zinc-800/80 bg-zinc-950/55 p-4 backdrop-blur-xl"
    };
    let input_wrap_class = if compact {
        "chat-input-wrap flex min-w-0 w-full items-end gap-2 rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.8)] p-2 shadow-[0_0_0_1px_rgba(255,255,255,0.02)]"
    } else {
        CHAT_COMPOSER_CLASS
    };
    let appearing_message_ids_list = appearing_message_ids();
    let removing_message_ids_list = removing_message_ids();
    let rendered_messages = messages();
    let has_messages = !rendered_messages.is_empty();
    let mut previous_day_key = None;
    let message_groups = group_consecutive_messages(&rendered_messages)
        .into_iter()
        .filter_map(|group| {
            let first_message = group.first()?;
            let day_key = message_day_key(&first_message.created_at);
            let date_label = (previous_day_key.as_ref() != Some(&day_key))
                .then(|| friendly_message_date(&first_message.created_at));
            previous_day_key = Some(day_key);
            Some((first_message.id.clone(), date_label, group))
        })
        .collect::<Vec<_>>();

    use_hook(move || {
        load_initial_history_when_connected(history_target, history_state);
    });

    use_hook(move || {
        let realtime = event_realtime.clone();
        spawn(async move {
            let mut receiver = realtime::subscribe_text_chat(&realtime);
            while let Some(event) = receiver.next().await {
                match event {
                    TextChatEvent::MessageCreated(message) => {
                        if message.room_id == event_room_id
                            && append_message(&mut messages, &mut appearing_message_ids, message)
                            && is_near_bottom()
                        {
                            pending_scroll.set(Some(ScrollCommand::Bottom));
                        }
                    }
                    TextChatEvent::MessageDeleted(payload) => {
                        if payload.room_id == event_room_id {
                            let message_id = payload.message_id.clone();
                            removing_message_ids.write().push(message_id.clone());
                            spawn(async move {
                                sleep_duration(Duration::from_millis(220)).await;
                                remove_message(&mut messages, &message_id);
                                removing_message_ids.write().retain(|id| id != &message_id);
                            });
                        }
                    }
                }
            }
        });
    });

    use_effect(move || {
        let _message_count = messages.len();
        let Some(command) = pending_scroll() else {
            return;
        };
        pending_scroll.set(None);
        let Some(element) = list_element.cloned() else {
            return;
        };

        spawn(async move {
            apply_scroll_command(element, command).await;
        });
    });

    let can_send = can_send_message(
        &draft(),
        pending_attachment().is_some(),
        is_sending() || is_selecting_image() || is_reading_clipboard(),
    );
    let submit_realtime = send_realtime.clone();
    let submit_server_id = send_server_id.clone();
    let submit_room_id = send_room_id.clone();
    let submit_component_current = component_current.clone();
    let submit_message = use_callback(move |_| {
        if can_send_message(
            &draft(),
            pending_attachment().is_some(),
            is_sending() || is_selecting_image() || is_reading_clipboard(),
        ) {
            refocus_requested.set(true);
            send_current_message(
                submit_realtime.clone(),
                submit_server_id.clone(),
                submit_room_id.clone(),
                compose_state,
                EventHandler::new({
                    let component_current = submit_component_current.clone();
                    move |_| {
                        restore_compose_input_focus(
                            compose_input_element,
                            refocus_requested,
                            component_current.clone(),
                        );
                    }
                }),
            );
        }
    });
    let load_older = use_callback(move |_| {
        load_older_history(older_target.clone(), history_state);
    });
    let on_delete_message = use_callback(move |message_id: String| {
        let realtime = delete_realtime.clone();
        let server_id = delete_server_id.clone();
        let room_id = delete_room_id.clone();
        removing_message_ids.write().push(message_id.clone());
        spawn(async move {
            let _ =
                realtime::delete_text_message(&realtime, server_id, room_id, message_id.clone())
                    .await;
            sleep_duration(Duration::from_millis(220)).await;
            remove_message(&mut messages, &message_id);
            removing_message_ids.write().retain(|id| id != &message_id);
        });
    });
    let add_pending_image = use_callback(move |result: Result<PendingImageAttachment, String>| {
        add_pending_image(room_compose_state, result);
    });
    let select_pending_image = use_callback(move |outcome: ImagePickerOutcome| {
        let result = match outcome {
            ImagePickerOutcome::Selected(PickedImage { file_name, bytes }) => {
                pending_image_attachment(file_name, bytes, MAX_CHAT_IMAGE_BYTES)
            }
            ImagePickerOutcome::Failed(error) => Err(error),
        };
        add_pending_image.call(result);
    });
    let clipboard_outcome = use_callback(move |result: Result<PendingImageAttachment, String>| {
        is_reading_clipboard.set(false);
        add_pending_image.call(result);
    });

    rsx! {
        div { class: "flex h-full min-h-0 flex-col",
            div {
                class: list_class,
                onmounted: move |event| list_element.set(Some(event.data.clone())),
                onscroll: move |_| {
                    if let Some(element) = list_element.cloned() {
                        spawn(async move {
                            update_scroll_state(
                                element,
                                is_near_bottom,
                                has_more,
                                older_loading,
                                initial_loading,
                                load_older,
                            ).await;
                        });
                    }
                },
                div { class: inner_class,
                    if older_loading() {
                        div { class: "flex justify-center py-2",
                            div { class: "h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-400" }
                        }
                    } else if let Some(error) = older_error() {
                        div { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-center text-[12px] leading-5 text-red-200",
                            p { "{error}" }
                            button {
                                r#type: "button",
                                class: "mt-2 rounded-lg border border-red-300/20 px-3 py-1 text-[12px] font-medium text-red-100 transition-colors hover:border-red-200/40 hover:bg-red-400/10",
                                onclick: move |_| load_older.call(()),
                                "Повторить"
                            }
                        }
                    }
                    if initial_loading() && !has_messages {
                        div { class: "space-y-3",
                            div { class: "h-14 animate-pulse rounded-2xl bg-zinc-900/80" }
                            div { class: "h-14 animate-pulse rounded-2xl bg-zinc-900/60" }
                            div { class: "h-14 animate-pulse rounded-2xl bg-zinc-900/40" }
                        }
                    } else if let Some(error) = history_error() {
                        div { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-3 text-center text-[12px] leading-5 text-red-200",
                            p { "{error}" }
                            button {
                                r#type: "button",
                                class: "mt-2 rounded-lg border border-red-300/20 px-3 py-1 text-[12px] font-medium text-red-100 transition-colors hover:border-red-200/40 hover:bg-red-400/10",
                                onclick: move |_| {
                                    load_initial_history(
                                        HistoryTarget {
                                            realtime: realtime.clone(),
                                            server_id: server_id.clone(),
                                            room_id: room.id.clone(),
                                        },
                                        history_state,
                                    );
                                },
                                "Повторить"
                            }
                        }
                    } else if !has_messages {
                        div { class: "rounded-[20px] border border-zinc-800 bg-zinc-900/60 p-6 text-center",
                            p { class: "text-[13px] font-medium text-zinc-100", "Сообщений пока нет" }
                            p { class: "mt-1 text-[12px] leading-5 text-zinc-500",
                                "Напиши первое сообщение в этой комнате."
                            }
                        }
                    } else {
                        for (group_key, date_label, group) in message_groups.iter().cloned() {
                            div { key: "{group_key}", class: "contents",
                                if let Some(label) = date_label {
                                    ChatMessageDateDivider { label }
                                }
                                ChatMessageGroup {
                                    messages: group,
                                    appearing_message_ids: appearing_message_ids_list.clone(),
                                    removing_message_ids: removing_message_ids_list.clone(),
                                    can_delete_messages: permissions.can_delete_messages,
                                    on_delete: move |id| on_delete_message.call(id),
                                    server_id: server_id.clone(),
                                    room_id: room.id.clone(),
                                }
                            }
                        }
                    }
                }
            }
            div { class: "relative",
                if !is_near_bottom() && has_messages {
                    div { class: "pointer-events-none absolute bottom-3 right-4 z-20",
                    button {
                        r#type: "button",
                        class: "group pointer-events-auto relative flex h-10 w-10 items-center justify-center rounded-full border border-zinc-800 bg-zinc-950/85 text-blue-200 shadow-[0_8px_22px_rgba(0,0,0,0.35)] transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-white/15 hover:bg-zinc-900/90 hover:text-blue-100",
                        "aria-label": "Перейти к последнему сообщению",
                        onclick: move |_| pending_scroll.set(Some(ScrollCommand::SmoothBottom)),
                        span { class: "pointer-events-none absolute bottom-[calc(100%+8px)] right-0 whitespace-nowrap rounded-lg border border-zinc-800 bg-zinc-950/95 px-2 py-1 text-[11px] font-medium text-zinc-300 opacity-0 shadow-[0_8px_22px_rgba(0,0,0,0.35)] transition-[opacity,transform] duration-150 group-hover:opacity-100",
                            "К последнему сообщению"
                        }
                        svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 5v14m0 0 6-6m-6 6-6-6" }
                        }
                    }
                }
                }
            }
            div { class: input_outer_class,
                div { class: CHAT_COMPOSER_GROUP_CLASS,
                    if is_reading_clipboard() {
                        div { class: "flex items-center gap-2 px-2 text-[11px] text-zinc-400", role: "status", "aria-live": "polite",
                        span { class: "size-3 animate-spin rounded-full border-2 border-zinc-600 border-t-blue-300", "aria-hidden": "true" }
                        "Получаем изображение из буфера обмена…"
                        }
                    }
                    if let Some(attachment) = pending_attachment() {
                        ChatAttachmentPreview {
                            attachment,
                            busy: is_sending(),
                            on_remove: move |_| {
                                if !is_sending() {
                                    info!("removed pending text chat image");
                                    pending_attachment.set(None);
                                    status.set(String::new());
                                }
                            }
                        }
                    }
                    div { class: input_wrap_class,
                    ImagePickerButton {
                        disabled: is_sending() || is_reading_clipboard() || pending_attachment().is_some(),
                        busy: is_selecting_image() || is_reading_clipboard(),
                        max_bytes: MAX_CHAT_IMAGE_BYTES,
                        on_outcome: move |outcome| select_pending_image.call(outcome),
                        on_active_change: move |active| is_selecting_image.set(active),
                    }
                    textarea {
                        rows: "1",
                        value: "{draft()}",
                        readonly: is_sending(),
                        placeholder: "Сообщение в {placeholder_prefix} {room.name}",
                        class: "max-h-28 min-h-10 min-w-0 flex-1 resize-none bg-transparent px-2 py-2 text-[13px] text-zinc-100 outline-none placeholder:text-zinc-600",
                        onmounted: move |event| {
                            compose_input_element.set(Some(event.data.clone()));
                        },
                        oninput: move |event| draft.set(event.value()),
                        onblur: move |_| refocus_requested.set(false),
                        onpaste: move |event| {
                            if !is_sending()
                                && !is_selecting_image()
                                && !is_reading_clipboard()
                                && pending_attachment().is_none()
                                && clipboard::read_pasted_image(event, clipboard_outcome)
                            {
                                is_reading_clipboard.set(true);
                            }
                        },
                        onkeydown: move |event| {
                            if event.key() == Key::Enter && !event.modifiers().shift() {
                                event.prevent_default();
                                submit_message.call(());
                            }
                        },
                    }
                    button {
                        r#type: "button",
                        disabled: !can_send,
                        class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-accent text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_4px_18px_rgba(59,130,246,0.16)] transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-blue-400 disabled:cursor-not-allowed disabled:opacity-45 disabled:hover:translate-y-0 disabled:hover:bg-accent",
                        "aria-label": "Отправить сообщение",
                        onpointerdown: move |event| {
                            event.prevent_default();
                            submit_message.call(());
                        },
                        onclick: move |_| {
                            submit_message.call(());
                        },
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 12 3.269 3.126A59.77 59.77 0 0 1 21.485 12 59.768 59.768 0 0 1 3.27 20.876L6 12Zm0 0h7.5" }
                        }
                    }
                    }
                    if !status().is_empty() {
                        p { class: "px-2 text-[11px] leading-4 text-red-200", "aria-live": "polite",
                            "{status()}"
                        }
                    }
                }
            }
        }
    }
}

fn restore_compose_input_focus(
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
            debug!(?error, "failed to restore text chat input focus");
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

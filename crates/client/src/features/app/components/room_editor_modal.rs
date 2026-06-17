//! Модальное окно создания и редактирования комнаты.

use cheenhub_contracts::rest::{ServerRoomKind, ServerRoomSummary};
use dioxus::prelude::*;

use crate::features::app::api;

use super::modal::Modal;

/// Рендерит поток создания и редактирования комнаты.
#[component]
pub(crate) fn RoomEditorModal(
    server_id: String,
    room: Option<ServerRoomSummary>,
    on_close: EventHandler<()>,
    on_saved: EventHandler<ServerRoomSummary>,
) -> Element {
    let initial_name = room
        .as_ref()
        .map(|room| room.name.clone())
        .unwrap_or_default();
    let initial_kind = room
        .as_ref()
        .map(|room| room.kind)
        .unwrap_or(ServerRoomKind::TextAndVoice);
    let room_id = room.as_ref().map(|room| room.id.clone());
    let title = if room_id.is_some() {
        "Изменить комнату"
    } else {
        "Создать комнату"
    };
    let action_label = if room_id.is_some() {
        "Сохранить"
    } else {
        "Создать"
    };
    let busy_label = if room_id.is_some() {
        "Сохраняем..."
    } else {
        "Создаем..."
    };
    let mut name = use_signal(|| initial_name);
    let mut kind = use_signal(|| initial_kind);
    let mut status = use_signal(String::new);
    let mut is_busy = use_signal(|| false);

    rsx! {
        Modal {
            title,
            on_close,
            form { class: "space-y-4",
                label { class: "block",
                    span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Название" }
                    input {
                        r#type: "text",
                        name: "room-name",
                        placeholder: "Например, общий",
                        value: name(),
                        maxlength: "48",
                        autocomplete: "off",
                        oninput: move |event| name.set(event.value()),
                        class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[14px] text-zinc-100 outline-none transition placeholder:text-zinc-700 focus:border-accent/70 focus:ring-4 focus:ring-accent/10"
                    }
                }

                label { class: "block",
                    span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Тип комнаты" }
                    select {
                        name: "room-kind",
                        value: room_kind_value(kind()),
                        onchange: move |event| kind.set(parse_room_kind(&event.value())),
                        class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[14px] text-zinc-100 outline-none transition focus:border-accent/70 focus:ring-4 focus:ring-accent/10",
                        option { value: "text_and_voice", "Текст и голос" }
                        option { value: "text", "Только текст" }
                        option { value: "voice", "Только голос" }
                    }
                }

                if !status().is_empty() {
                    p { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
                        "{status()}"
                    }
                }

                div { class: "flex justify-end gap-2 pt-1",
                    button {
                        r#type: "button",
                        disabled: is_busy(),
                        class: "flex h-10 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 px-4 text-[13px] font-medium text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100 disabled:cursor-not-allowed disabled:opacity-60",
                        onclick: move |_| on_close.call(()),
                        "Отмена"
                    }
                    button {
                        r#type: "button",
                        disabled: is_busy(),
                        class: "flex h-10 items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)] transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-blue-400 disabled:cursor-not-allowed disabled:opacity-60",
                        onclick: move |_| {
                            if is_busy() {
                                return;
                            }
                            is_busy.set(true);
                            status.set(String::new());
                            let request_server_id = server_id.clone();
                            let request_room_id = room_id.clone();
                            let request_name = name();
                            let request_kind = kind();

                            spawn(async move {
                                let result = if let Some(room_id) = request_room_id {
                                    api::update_server_room(
                                        request_server_id,
                                        room_id,
                                        request_name,
                                        request_kind,
                                    )
                                    .await
                                } else {
                                    api::create_server_room(
                                        request_server_id,
                                        request_name,
                                        request_kind,
                                    )
                                    .await
                                };

                                match result {
                                    Ok(room) => {
                                        on_saved.call(room);
                                        on_close.call(());
                                    }
                                    Err(error) => {
                                        status.set(error);
                                        is_busy.set(false);
                                    }
                                }
                            });
                        },
                        if is_busy() { "{busy_label}" } else { "{action_label}" }
                    }
                }
            }
        }
    }
}

fn parse_room_kind(value: &str) -> ServerRoomKind {
    match value {
        "text" => ServerRoomKind::Text,
        "voice" => ServerRoomKind::Voice,
        _ => ServerRoomKind::TextAndVoice,
    }
}

fn room_kind_value(kind: ServerRoomKind) -> &'static str {
    match kind {
        ServerRoomKind::Text => "text",
        ServerRoomKind::Voice => "voice",
        ServerRoomKind::TextAndVoice => "text_and_voice",
    }
}

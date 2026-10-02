//! Диалог удаления комнаты вместе с запуском операции удаления.
//!
//! Существует отдельно от `ServerRoomsScope`, чтобы сам сайдбар не содержал
//! последовательность «показать диалог → отправить запрос → применить результат».

use cheenhub_contracts::rest::ServerRoomSummary;
use dioxus::prelude::*;

use super::room_delete_confirm::RoomDeleteConfirm;
use super::server_rooms_delete::{RoomSidebarContext, delete_room_and_switch};

/// Показывает подтверждение удаления и выполняет удаление комнаты после согласия.
///
/// Компонент вызывается только при установленном `room`, поэтому пользователь видит
/// имя конкретной комнаты, а не абстрактное действие. Пока запрос выполняется, диалог
/// блокирует повторное подтверждение, чтобы не отправить два DELETE подряд.
pub(super) fn render_room_delete_flow(
    room: ServerRoomSummary,
    server_id: String,
    mut pending_delete_room: Signal<Option<ServerRoomSummary>>,
    mut deleting_room_id: Signal<Option<String>>,
    mut room_action_status: Signal<String>,
    sidebar_context: RoomSidebarContext,
) -> Element {
    let is_deleting = deleting_room_id().is_some();
    // Навигатор берётся из контекста навигации: тип `Navigator` не сравним,
    // поэтому передавать его пропом компонента нельзя.
    let navigator = use_navigator();

    rsx! {
        RoomDeleteConfirm {
            room_name: room.name.clone(),
            is_deleting,
            on_cancel: move |_| {
                // Пока идёт запрос, закрывать диалог нельзя: пользователь потеряет
                // обратную связь о выполняющейся операции.
                if deleting_room_id().is_some() {
                    return;
                }
                pending_delete_room.set(None);
            },
            on_confirm: {
                let confirm_server_id = server_id.clone();
                let confirm_room_id = room.id.clone();
                move |_| {
                    if deleting_room_id().is_some() {
                        return;
                    }
                    room_action_status.set(String::new());
                    pending_delete_room.set(None);
                    deleting_room_id.set(Some(confirm_room_id.clone()));
                    info!(
                        server_id = %confirm_server_id,
                        room_id = %confirm_room_id,
                        "deleting server room"
                    );
                    let request_server_id = confirm_server_id.clone();
                    let request_room_id = confirm_room_id.clone();
                    let mut status = room_action_status;
                    let mut deleting = deleting_room_id;
                    spawn(async move {
                        let result = delete_room_and_switch(
                            request_server_id,
                            request_room_id,
                            sidebar_context,
                            navigator,
                        )
                        .await;
                        if let Err(error) = result {
                            status.set(error);
                        }
                        deleting.set(None);
                    });
                }
            },
        }
    }
}

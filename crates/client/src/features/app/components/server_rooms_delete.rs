//! Удаление комнаты сервера и переход к соседней комнате.

use cheenhub_contracts::rest::ServerRoomSummary;
use dioxus::prelude::*;
use dioxus::router::Navigator;

use crate::Route;
use crate::features::app::api;

use super::app_shell::{ActiveRoom, ServerShellState, room_kind_attr};
use super::server_rooms_state::{
    ServerWorkspace, active_room, chat_open_for_room, ensure_workspace_mounted,
};

/// Определяет комнату, которую нужно открыть после удаления `room_id`.
///
/// Если удалена активная комната, выбирается первая из оставшихся; иначе сохраняется
/// прежняя активная комната, чтобы пользователь не покидал открытый чат без причины.
pub(super) fn next_active_room_after_delete(
    remaining_rooms: &[ServerRoomSummary],
    deleted_room_id: &str,
    current_room_id: Option<&str>,
) -> Option<String> {
    if current_room_id == Some(deleted_room_id) {
        return remaining_rooms.first().map(|room| room.id.clone());
    }

    current_room_id.map(ToOwned::to_owned)
}

/// Освобождает workspace удалённой комнаты и переключает активный workspace на следующую комнату.
///
/// Возвращает обновлённый список смонтированных workspace и новый активный workspace,
/// чтобы вызывающий код применил их одной операцией.
pub(super) fn workspace_after_room_delete(
    mounted_workspaces: Vec<ServerWorkspace>,
    active_workspace: Option<ServerWorkspace>,
    deleted_room_id: &str,
    next_room_id: Option<&str>,
) -> (Vec<ServerWorkspace>, Option<ServerWorkspace>) {
    let mut next_mounted_workspaces = mounted_workspaces;
    next_mounted_workspaces.retain(
        |workspace| !matches!(workspace, ServerWorkspace::Room(id) if id == deleted_room_id),
    );

    let deleted_room_was_active =
        matches!(active_workspace, Some(ServerWorkspace::Room(ref id)) if id == deleted_room_id);
    if !deleted_room_was_active {
        return (next_mounted_workspaces, active_workspace);
    }

    let next_workspace = next_room_id.map(|room_id| ServerWorkspace::Room(room_id.to_owned()));
    if let Some(workspace) = next_workspace.clone() {
        ensure_workspace_mounted(&mut next_mounted_workspaces, workspace);
    }

    (next_mounted_workspaces, next_workspace)
}

/// Формирует состояние оболочки для комнаты, которая станет активной после удаления.
///
/// Возвращает идентификатор следующей комнаты вместе с её состоянием оболочки, чтобы
/// вызывающий код обновил оболочку и маршрут без повторного поиска комнаты.
pub(super) fn shell_state_after_delete(
    remaining_rooms: &[ServerRoomSummary],
    next_room_id: Option<&str>,
    chat_open_by_room: &[(String, bool)],
) -> Option<(String, ServerShellState, ActiveRoom)> {
    let next_room = active_room(remaining_rooms, next_room_id)?;

    Some((
        next_room.id.clone(),
        ServerShellState {
            chat_open: chat_open_for_room(chat_open_by_room, &next_room.id),
            room_kind: room_kind_attr(next_room.kind),
        },
        next_room,
    ))
}

/// Состояние сайдбара, которое операции над комнатами читают и изменяют.
///
/// Собирается один раз в компоненте сайдбара и передаётся операциям, поэтому
/// сигналы комнат не нужно перечислять в каждой сигнатуре. Все поля копируемые,
/// поэтому контекст можно свободно захватывать в обработчики событий.
/// Навигатор сюда не входит: он доступен через контекст навигации в области вызова.
#[derive(Clone, Copy)]
pub(super) struct RoomSidebarContext {
    /// Список комнат, хранящийся в ресурсе загрузки.
    pub(super) rooms: Resource<Result<Vec<ServerRoomSummary>, String>>,
    /// Активная комната, выбранная пользователем.
    pub(super) active_room_id: Signal<Option<String>>,
    /// Workspace, смонтированные для этого сервера.
    pub(super) mounted_workspaces: Signal<Vec<ServerWorkspace>>,
    /// Workspace, открытый для пользователя прямо сейчас.
    pub(super) active_workspace: Signal<Option<ServerWorkspace>>,
    /// Признак открытости текстового чата по комнатам.
    pub(super) chat_open_by_room: Signal<Vec<(String, bool)>>,
    /// Сообщает оболочке приложения о смене состояния чата.
    pub(super) on_state_change: EventHandler<(String, ServerShellState)>,
}

/// Выполняет удаление комнаты и переводит рабочую область на следующую комнату.
///
/// Запрос, обновление локального списка, пересборка workspace и замена маршрута
/// выполняются целиком здесь, чтобы вызывающий компонент не дублировал эту
/// последовательность. Ошибку запроса возвращает вызывающему коду, чтобы он показал
/// её в сайдбаре и вернул управление комнатами пользователю.
pub(super) async fn delete_room_and_switch(
    server_id: String,
    room_id: String,
    context: RoomSidebarContext,
    navigator: Navigator,
) -> Result<(), String> {
    let RoomSidebarContext {
        mut rooms,
        mut active_room_id,
        mut mounted_workspaces,
        mut active_workspace,
        chat_open_by_room,
        on_state_change,
    } = context;
    let known_rooms = match rooms.read().clone() {
        Some(Ok(known_rooms)) => known_rooms,
        _ => Vec::new(),
    };

    let result = api::delete_server_room(server_id.clone(), room_id.clone()).await;
    let next_rooms = match result {
        Ok(()) => {
            let mut next_rooms = known_rooms;
            next_rooms.retain(|room| room.id != room_id);
            info!(
                %server_id,
                %room_id,
                remaining_rooms = next_rooms.len(),
                "deleted server room"
            );
            next_rooms
        }
        Err(error) => {
            warn!(
                %server_id,
                %room_id,
                %error,
                "failed to delete server room"
            );
            return Err(error);
        }
    };

    let next_active_room_id =
        next_active_room_after_delete(&next_rooms, &room_id, active_room_id().as_deref());
    active_room_id.set(next_active_room_id.clone());
    *rooms.write() = Some(Ok(next_rooms.clone()));

    let (next_mounted_workspaces, next_active_workspace) = workspace_after_room_delete(
        mounted_workspaces(),
        active_workspace(),
        &room_id,
        next_active_room_id.as_deref(),
    );
    mounted_workspaces.set(next_mounted_workspaces);
    active_workspace.set(next_active_workspace);

    match shell_state_after_delete(
        &next_rooms,
        next_active_room_id.as_deref(),
        &chat_open_by_room(),
    ) {
        Some((_, state, next_room)) => {
            on_state_change.call((server_id.clone(), state));
            navigator.replace(Route::AppServerRoom {
                server_id,
                room_id: next_room.id,
            });
        }
        None => {
            info!(%server_id, "deleted the last room of the server");
            navigator.replace(Route::AppServer { server_id });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{next_active_room_after_delete, workspace_after_room_delete};
    use crate::features::app::components::server_rooms_state::ServerWorkspace;
    use cheenhub_contracts::rest::ServerRoomSummary;

    fn room(id: &str) -> ServerRoomSummary {
        ServerRoomSummary {
            id: id.to_owned(),
            name: id.to_owned(),
            kind: cheenhub_contracts::rest::ServerRoomKind::Text,
            position: 0,
            write_access: Default::default(),
            can_write: true,
        }
    }

    #[test]
    fn deleting_active_room_selects_first_remaining_room() {
        let remaining = vec![room("first"), room("second")];

        let next = next_active_room_after_delete(&remaining, "first", Some("first"));

        assert_eq!(next.as_deref(), Some("first"));
    }

    #[test]
    fn deleting_other_room_keeps_current_selection() {
        let remaining = vec![room("first")];

        let next = next_active_room_after_delete(&remaining, "other", Some("first"));

        assert_eq!(next.as_deref(), Some("first"));
    }

    #[test]
    fn deleting_last_room_clears_active_workspace() {
        let mounted = vec![ServerWorkspace::Room("first".to_owned())];
        let active = Some(ServerWorkspace::Room("first".to_owned()));

        let (next_mounted, next_active) =
            workspace_after_room_delete(mounted, active, "first", None);

        assert!(next_mounted.is_empty());
        assert_eq!(next_active, None);
    }

    #[test]
    fn deleting_inactive_room_keeps_active_workspace() {
        let mounted = vec![
            ServerWorkspace::Room("first".to_owned()),
            ServerWorkspace::Room("second".to_owned()),
        ];
        let active = Some(ServerWorkspace::Room("first".to_owned()));

        let (next_mounted, next_active) =
            workspace_after_room_delete(mounted, active, "second", Some("first"));

        assert_eq!(
            next_mounted,
            vec![ServerWorkspace::Room("first".to_owned())]
        );
        assert_eq!(next_active, Some(ServerWorkspace::Room("first".to_owned())));
    }
}

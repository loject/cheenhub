//! Обработка действий контекстного меню сервера.

use dioxus::logger::tracing::info;
use dioxus::prelude::*;

use super::app_shell::AppModal;
use super::server_context_menu::ServerMenuAction;
use super::server_rooms_state::{ServerWorkspace, open_server_settings_workspace};

/// Состояние сайдбара, которое меняют действия контекстного меню сервера.
///
/// Сигналы копируемые, поэтому контекст можно свободно захватывать в обработчики.
/// Навигация не входит в контекст: она нужна только для перехода после удаления сервера,
/// который выполняет оболочка приложения.
#[derive(Clone, Copy)]
pub(super) struct ServerMenuScope {
    /// Открыто ли сейчас контекстное меню сервера.
    pub(super) is_menu_open: Signal<bool>,
    /// Workspace, смонтированные для этого сервера.
    pub(super) mounted_workspaces: Signal<Vec<ServerWorkspace>>,
    /// Workspace, открытый для пользователя прямо сейчас.
    pub(super) active_workspace: Signal<Option<ServerWorkspace>>,
    /// Открыта ли мобильная рабочая область этого сервера.
    pub(super) mobile_workspace_open: Signal<bool>,
    /// Сообщает оболочке приложения о запросе открыть модальное окно.
    pub(super) on_open_modal: EventHandler<AppModal>,
}

/// Применяет действие меню к состоянию сайдбара и оболочки приложения.
///
/// Меню закрывается сразу: показывать его поверх открытой модалки или во время
/// выполняющегося запроса не нужно.
pub(super) fn apply_server_menu_action(
    action: ServerMenuAction,
    menu_server_id: String,
    invite_server_name: String,
    context: ServerMenuScope,
) {
    let ServerMenuScope {
        mut is_menu_open,
        mounted_workspaces,
        active_workspace,
        mobile_workspace_open,
        on_open_modal,
    } = context;
    is_menu_open.set(false);

    match action {
        ServerMenuAction::OpenSettings => {
            info!(
                server_id = %menu_server_id,
                "opened server settings workspace"
            );
            open_server_settings_workspace(
                mounted_workspaces,
                active_workspace,
                mobile_workspace_open,
            );
        }
        ServerMenuAction::CreateInvite => {
            info!(
                server_id = %menu_server_id,
                "opened server invite modal from context menu"
            );
            on_open_modal.call(AppModal::InviteLink {
                server_id: menu_server_id,
                server_name: invite_server_name,
            });
        }
        ServerMenuAction::LeaveServer => {
            info!(server_id = %menu_server_id, "opened leave server confirmation");
            on_open_modal.call(AppModal::LeaveServer {
                server_id: menu_server_id,
                server_name: invite_server_name,
            });
        }
    }
}

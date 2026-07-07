//! Маршрут рабочей области приложения.

use crate::Route;

/// Выбранная рабочая область внутри авторизованного приложения.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AppWorkspaceRoute {
    /// Страница друзей без выбранного личного диалога.
    Friends,
    /// Открытый личный диалог.
    DirectMessage {
        /// Идентификатор личного диалога.
        conversation_id: String,
    },
    /// Сервер без конкретной комнаты; после загрузки комнат маршрут уточняется.
    Server {
        /// Идентификатор сервера.
        server_id: String,
    },
    /// Конкретная комната на сервере.
    ServerRoom {
        /// Идентификатор сервера.
        server_id: String,
        /// Идентификатор комнаты.
        room_id: String,
    },
}

impl AppWorkspaceRoute {
    /// Возвращает рабочую область для маршрута верхнего уровня.
    pub(crate) fn from_route(route: &Route) -> Option<Self> {
        match route {
            Route::AppFriends {} => Some(Self::Friends),
            Route::AppDirectMessage { conversation_id } => Some(Self::DirectMessage {
                conversation_id: conversation_id.clone(),
            }),
            Route::AppServer { server_id } => Some(Self::Server {
                server_id: server_id.clone(),
            }),
            Route::AppServerRoom { server_id, room_id } => Some(Self::ServerRoom {
                server_id: server_id.clone(),
                room_id: room_id.clone(),
            }),
            _ => None,
        }
    }

    /// Возвращает идентификатор активного сервера, если открыт серверный workspace.
    pub(crate) fn server_id(&self) -> Option<&str> {
        match self {
            Self::Server { server_id } | Self::ServerRoom { server_id, .. } => Some(server_id),
            Self::Friends | Self::DirectMessage { .. } => None,
        }
    }

    /// Возвращает идентификатор комнаты из маршрута, если он есть.
    pub(crate) fn room_id(&self) -> Option<&str> {
        match self {
            Self::ServerRoom { room_id, .. } => Some(room_id),
            Self::Friends | Self::DirectMessage { .. } | Self::Server { .. } => None,
        }
    }

    /// Возвращает идентификатор личного диалога из маршрута, если он есть.
    pub(crate) fn conversation_id(&self) -> Option<&str> {
        match self {
            Self::DirectMessage { conversation_id } => Some(conversation_id),
            Self::Friends | Self::Server { .. } | Self::ServerRoom { .. } => None,
        }
    }

    /// Проверяет, что маршрут относится к social workspace.
    pub(crate) fn is_social(&self) -> bool {
        matches!(self, Self::Friends | Self::DirectMessage { .. })
    }
}

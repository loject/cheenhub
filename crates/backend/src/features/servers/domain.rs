//! Доменные модели сервера.

use cheenhub_contracts::realtime::{ServerRoleKind, ServerRolePermission};
use cheenhub_contracts::rest::{ServerRoomKind, ServerRoomWriteAccessMode};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Максимальное количество действующих ссылок-приглашений, которое может создать один пользователь на одном сервере.
///
/// Сейчас значение фиксировано для всех пользователей и серверов. В следующих итерациях лимит станет
/// зависеть от роли пользователя и будет настраиваться владельцем сервера индивидуально для каждой роли,
/// а пользователю будет возвращаться максимум среди ролей, которые ему назначены.
pub(crate) const MAX_OWN_INVITE_LINKS: u32 = 10;

/// Данные сервера, используемые в потоках сервера.
#[derive(Debug, Clone)]
pub(crate) struct Server {
    /// Stable server identifier.
    pub(crate) id: Uuid,
    /// User that owns the server.
    pub(crate) owner_user_id: Uuid,
    /// Human-readable server name.
    pub(crate) name: String,
    /// Stored server avatar image identifier.
    pub(crate) avatar_image_id: Option<Uuid>,
    /// Target Opus voice audio bitrate in bits per second.
    pub(crate) audio_bitrate_bps: u32,
    /// Server creation timestamp.
    #[allow(dead_code)]
    pub(crate) created_at: DateTime<Utc>,
    /// Last server update timestamp.
    #[allow(dead_code)]
    pub(crate) updated_at: DateTime<Utc>,
}

/// Данные сервера с контекстом членства текущего пользователя.
#[derive(Debug, Clone)]
pub(crate) struct ServerAccess {
    /// Server available to the current user.
    pub(crate) server: Server,
    /// Whether the current user is an active server member.
    pub(crate) is_member: bool,
}

/// Настройка доступа к записи в комнату сервера.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServerRoomWriteAccess {
    /// Режим доступа к записи.
    pub(crate) mode: ServerRoomWriteAccessMode,
    /// Идентификаторы ролей, которым разрешено писать, в режиме `SelectedRoles`.
    pub(crate) role_ids: Vec<Uuid>,
}

impl ServerRoomWriteAccess {
    /// Создает настройку доступа, где писать могут все участники сервера.
    pub(crate) fn all_members() -> Self {
        Self {
            mode: ServerRoomWriteAccessMode::AllMembers,
            role_ids: Vec::new(),
        }
    }
}

/// Данные комнаты сервера, используемые в потоках сервера.
#[derive(Debug, Clone)]
pub(crate) struct ServerRoom {
    /// Stable room identifier.
    pub(crate) id: Uuid,
    /// Server the room belongs to.
    pub(crate) server_id: Uuid,
    /// Human-readable room name.
    pub(crate) name: String,
    /// Room interaction type.
    pub(crate) kind: ServerRoomKind,
    /// Append-only ordering position inside the server.
    pub(crate) position: u32,
    /// Room write access settings.
    pub(crate) write_access: ServerRoomWriteAccess,
    /// Room creation timestamp.
    #[allow(dead_code)]
    pub(crate) created_at: DateTime<Utc>,
    /// Last room update timestamp.
    pub(crate) updated_at: DateTime<Utc>,
}

/// Данные роли сервера, используемые в потоках настроек сервера.
#[derive(Debug, Clone)]
pub(crate) struct ServerRole {
    /// Stable role identifier.
    pub(crate) id: Uuid,
    /// Server the role belongs to.
    pub(crate) server_id: Uuid,
    /// Human-readable role name.
    pub(crate) name: String,
    /// Hex role color.
    pub(crate) color: String,
    /// Role kind.
    pub(crate) kind: ServerRoleKind,
    /// Ordering position inside the server.
    pub(crate) position: u32,
    /// Enabled permissions.
    pub(crate) permissions: Vec<ServerRolePermission>,
    /// Role creation timestamp.
    #[allow(dead_code)]
    pub(crate) created_at: DateTime<Utc>,
    /// Last role update timestamp.
    #[allow(dead_code)]
    pub(crate) updated_at: DateTime<Utc>,
}

/// Данные приглашения сервера, используемые в потоках сервера.
#[derive(Debug, Clone)]
pub(crate) struct ServerInvite {
    /// Stable invite identifier used as the invite code.
    pub(crate) id: Uuid,
    /// Server the invite belongs to.
    pub(crate) server_id: Uuid,
    /// User that created the invite.
    pub(crate) creator_user_id: Uuid,
    /// Optional maximum number of accepted invite uses.
    pub(crate) max_uses: Option<u32>,
    /// Optional invite expiration timestamp.
    pub(crate) expires_at: Option<DateTime<Utc>>,
    /// Invite creation timestamp.
    pub(crate) created_at: DateTime<Utc>,
    /// Invite revocation timestamp.
    pub(crate) revoked_at: Option<DateTime<Utc>>,
    /// Invite soft-delete timestamp. The invite stops being usable, but stays stored together
    /// with its uses so that it remains possible to tell which invite brought a member in.
    pub(crate) deleted_at: Option<DateTime<Utc>>,
}

/// Данные участника сервера, используемые в потоках сервера.
#[derive(Debug, Clone)]
pub(crate) struct ServerMember {
    /// Stable server member row identifier.
    #[allow(dead_code)]
    pub(crate) id: Uuid,
    /// Server the member belongs to.
    pub(crate) server_id: Uuid,
    /// User that joined the server.
    pub(crate) user_id: Uuid,
    /// Membership start timestamp.
    #[allow(dead_code)]
    pub(crate) joined_at: DateTime<Utc>,
    /// Membership end timestamp for future soft leave.
    pub(crate) left_at: Option<DateTime<Utc>>,
}

/// Временное исключение с сервера, блокирующее повторное присоединение кикнутого пользователя.
#[derive(Debug, Clone)]
pub(crate) struct ServerMemberExclusion {
    /// Stable exclusion row identifier.
    #[allow(dead_code)]
    pub(crate) id: Uuid,
    /// Server the exclusion belongs to.
    pub(crate) server_id: Uuid,
    /// User blocked from rejoining.
    pub(crate) user_id: Uuid,
    /// User or system actor that created the exclusion.
    #[allow(dead_code)]
    pub(crate) initiator_user_id: Uuid,
    /// Timestamp until which the user cannot rejoin.
    pub(crate) expires_at: DateTime<Utc>,
    /// Exclusion creation timestamp.
    #[allow(dead_code)]
    pub(crate) created_at: DateTime<Utc>,
}

/// Данные использования приглашения сервера, используемые в потоках сервера.
#[derive(Debug, Clone)]
pub(crate) struct ServerInviteUse {
    /// Stable invite use row identifier.
    #[allow(dead_code)]
    pub(crate) id: Uuid,
    /// Invite that was used successfully.
    pub(crate) invite_id: Uuid,
    /// User that used the invite successfully.
    pub(crate) user_id: Uuid,
    /// Invite use timestamp.
    pub(crate) used_at: DateTime<Utc>,
}

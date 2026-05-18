//! Role settings data and styling helpers.

use cheenhub_contracts::realtime::{
    ServerRoleDraft, ServerRoleEntry, ServerRoleKind, ServerRolePermission,
};
use dioxus::prelude::{Signal, WritableExt};

pub(super) const ROLE_COLORS: &[&str] = &[
    "#3b82f6", "#22c55e", "#a855f7", "#f59e0b", "#ef4444", "#06b6d4", "#e879f9", "#f97316",
    "#94a3b8",
];

pub(super) const OWNER_ROLE_ID: &str = "2b38a96f-7e57-4411-8ab9-379d1eb8a001";
pub(super) const MEMBER_ROLE_ID: &str = "4ec49843-8ad0-4fdf-a94b-14207d71a002";

#[derive(Clone, PartialEq)]
pub(super) struct RoleDraft {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) color: String,
    pub(super) members: u32,
    pub(super) is_required: bool,
    pub(super) kind: ServerRoleKind,
    pub(super) permissions: Vec<RolePermission>,
}

impl RoleDraft {
    pub(super) fn is_owner(&self) -> bool {
        self.kind == ServerRoleKind::Owner
    }

    pub(super) fn description(&self) -> &'static str {
        if self.is_owner() {
            "Полный доступ ко всем настройкам сервера."
        } else if self.kind == ServerRoleKind::Member {
            "Базовая роль для обычных участников сервера."
        } else {
            "Настраиваемая роль с выбранным набором прав."
        }
    }

    pub(super) fn effective_permissions(&self) -> Vec<RolePermission> {
        if self.is_owner() {
            RolePermission::all().to_vec()
        } else {
            self.permissions.clone()
        }
    }

    pub(super) fn has_permission(&self, permission: RolePermission) -> bool {
        self.is_owner() || self.permissions.contains(&permission)
    }

    pub(super) fn set_permission(&mut self, permission: RolePermission, enabled: bool) {
        if self.is_owner() {
            self.permissions = RolePermission::all().to_vec();
            return;
        }

        if enabled && !self.permissions.contains(&permission) {
            self.permissions.push(permission);
        } else if !enabled {
            self.permissions.retain(|saved| *saved != permission);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RolePermission {
    CreateInviteLinks,
    KickServerMembers,
    ManageRoles,
    KickVoiceMembers,
    DeleteMessages,
}

impl RolePermission {
    pub(super) fn all() -> &'static [RolePermission] {
        &[
            RolePermission::CreateInviteLinks,
            RolePermission::KickServerMembers,
            RolePermission::ManageRoles,
            RolePermission::KickVoiceMembers,
            RolePermission::DeleteMessages,
        ]
    }

    pub(super) fn key(self) -> &'static str {
        match self {
            RolePermission::CreateInviteLinks => "create_invite_links",
            RolePermission::KickServerMembers => "kick_server_members",
            RolePermission::ManageRoles => "manage_roles",
            RolePermission::KickVoiceMembers => "kick_voice_members",
            RolePermission::DeleteMessages => "delete_messages",
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            RolePermission::CreateInviteLinks => "Создавать инвайт-ссылки",
            RolePermission::KickServerMembers => "Кикать с сервера",
            RolePermission::ManageRoles => "Управлять ролями",
            RolePermission::KickVoiceMembers => "Кикать из голосовой комнаты",
            RolePermission::DeleteMessages => "Удалять чужие сообщения",
        }
    }

    pub(super) fn hint(self) -> &'static str {
        match self {
            RolePermission::CreateInviteLinks => "Создание ссылок для входа новых участников.",
            RolePermission::KickServerMembers => {
                "Исключение участника с возможностью вернуться по приглашению."
            }
            RolePermission::ManageRoles => "Создание, удаление и изменение ролей ниже своей.",
            RolePermission::KickVoiceMembers => "Отключение участника от голосовой комнаты.",
            RolePermission::DeleteMessages => {
                "Удаление любых сообщений в текстовых комнатах сервера."
            }
        }
    }

    pub(super) fn from_realtime(permission: ServerRolePermission) -> Self {
        match permission {
            ServerRolePermission::CreateInviteLinks => RolePermission::CreateInviteLinks,
            ServerRolePermission::KickServerMembers => RolePermission::KickServerMembers,
            ServerRolePermission::ManageRoles => RolePermission::ManageRoles,
            ServerRolePermission::KickVoiceMembers => RolePermission::KickVoiceMembers,
            ServerRolePermission::DeleteMessages => RolePermission::DeleteMessages,
        }
    }

    pub(super) fn to_realtime(self) -> ServerRolePermission {
        match self {
            RolePermission::CreateInviteLinks => ServerRolePermission::CreateInviteLinks,
            RolePermission::KickServerMembers => ServerRolePermission::KickServerMembers,
            RolePermission::ManageRoles => ServerRolePermission::ManageRoles,
            RolePermission::KickVoiceMembers => ServerRolePermission::KickVoiceMembers,
            RolePermission::DeleteMessages => ServerRolePermission::DeleteMessages,
        }
    }
}

pub(super) fn initial_roles() -> Vec<RoleDraft> {
    vec![
        RoleDraft {
            id: OWNER_ROLE_ID.to_owned(),
            name: "Владелец".to_owned(),
            color: "#f59e0b".to_owned(),
            members: 1,
            is_required: true,
            kind: ServerRoleKind::Owner,
            permissions: RolePermission::all().to_vec(),
        },
        RoleDraft {
            id: "f3fe66a7-79f5-4d31-81dc-2fd43b70a003".to_owned(),
            name: "Модератор".to_owned(),
            color: "#3b82f6".to_owned(),
            members: 8,
            is_required: false,
            kind: ServerRoleKind::Custom,
            permissions: vec![
                RolePermission::CreateInviteLinks,
                RolePermission::KickServerMembers,
                RolePermission::KickVoiceMembers,
            ],
        },
        RoleDraft {
            id: MEMBER_ROLE_ID.to_owned(),
            name: "Участник".to_owned(),
            color: "#94a3b8".to_owned(),
            members: 142,
            is_required: true,
            kind: ServerRoleKind::Member,
            permissions: vec![RolePermission::CreateInviteLinks],
        },
    ]
}

pub(super) fn roles_from_realtime(roles: Vec<ServerRoleEntry>) -> Vec<RoleDraft> {
    roles
        .into_iter()
        .map(|role| RoleDraft {
            id: role.role_id,
            name: role.name,
            color: role.color,
            members: role.members,
            is_required: role.is_required,
            kind: role.kind,
            permissions: role
                .permissions
                .into_iter()
                .map(RolePermission::from_realtime)
                .collect(),
        })
        .collect()
}

pub(super) fn roles_to_realtime(roles: Vec<RoleDraft>) -> Vec<ServerRoleDraft> {
    roles
        .into_iter()
        .map(|role| {
            let permissions = role
                .effective_permissions()
                .into_iter()
                .map(RolePermission::to_realtime)
                .collect();

            ServerRoleDraft {
                role_id: Some(role.id),
                name: role.name,
                color: role.color,
                kind: role.kind,
                permissions,
            }
        })
        .collect()
}

pub(super) fn selected_role<'a>(
    roles: &'a [RoleDraft],
    selected_role_id: &str,
) -> Option<&'a RoleDraft> {
    roles
        .iter()
        .find(|role| role.id == selected_role_id)
        .or_else(|| roles.first())
}

pub(super) fn update_selected_role(
    roles: &mut Signal<Option<Vec<RoleDraft>>>,
    selected_role_id: &str,
    update: impl FnOnce(&mut RoleDraft),
) {
    let Some(mut next_roles) = roles() else {
        return;
    };
    if let Some(role) = next_roles
        .iter_mut()
        .find(|role| role.id == selected_role_id)
    {
        update(role);
    }
    roles.set(Some(next_roles));
}

pub(super) fn role_initial(name: &str) -> String {
    name.trim()
        .chars()
        .next()
        .map(|letter| letter.to_uppercase().to_string())
        .unwrap_or_else(|| "R".to_owned())
}

pub(super) fn non_empty_role_name(value: String) -> String {
    if value.trim().is_empty() {
        "Без названия".to_owned()
    } else {
        value
    }
}

pub(super) fn normalize_hex_input(value: String) -> String {
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        trimmed
    } else {
        format!("#{trimmed}")
    }
}

pub(super) fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };

    hex.len() == 6 && hex.chars().all(|character| character.is_ascii_hexdigit())
}

pub(super) fn preview_color(value: &str) -> String {
    if is_hex_color(value) {
        value.to_owned()
    } else {
        "transparent".to_owned()
    }
}

pub(super) fn hex_to_rgb(value: &str) -> String {
    if !is_hex_color(value) {
        return "148, 163, 184".to_owned();
    }

    let red = u8::from_str_radix(&value[1..3], 16).unwrap_or(148);
    let green = u8::from_str_radix(&value[3..5], 16).unwrap_or(163);
    let blue = u8::from_str_radix(&value[5..7], 16).unwrap_or(184);
    format!("{red}, {green}, {blue}")
}

pub(super) fn role_card_class(active: bool) -> &'static str {
    if active {
        "role-card w-full rounded-2xl border border-accent/25 bg-accent/10 p-3 text-left shadow-[0_10px_32px_rgba(59,130,246,0.08)] transition-[background,border-color,box-shadow,transform] duration-200 ease-out hover:-translate-y-0.5"
    } else {
        "role-card w-full rounded-2xl border border-zinc-800 bg-zinc-900/70 p-3 text-left transition-[background,border-color,box-shadow,transform] duration-200 ease-out hover:-translate-y-0.5 hover:border-accent/20 hover:bg-accent/5 hover:shadow-[0_12px_34px_rgba(0,0,0,0.18)]"
    }
}

pub(super) fn delete_button_class(disabled: bool) -> &'static str {
    if disabled {
        "rounded-xl border border-rose-900/40 bg-rose-950/20 px-3 py-2 text-[13px] font-medium text-rose-300/40 opacity-60"
    } else {
        "rounded-xl border border-rose-900/60 bg-rose-950/40 px-3 py-2 text-[13px] font-medium text-rose-300 transition-[background,border-color,color,transform] duration-150 hover:-translate-y-px hover:border-rose-800 hover:bg-rose-950/60 hover:text-rose-200"
    }
}

pub(super) fn color_button_class(active: bool) -> &'static str {
    if active {
        "grid h-9 w-9 place-items-center rounded-xl border border-zinc-500 bg-zinc-800 shadow-[0_0_0_3px_rgba(255,255,255,0.05)] transition-[background,border-color,box-shadow,transform] duration-150 hover:scale-105"
    } else {
        "grid h-9 w-9 place-items-center rounded-xl border border-zinc-800 bg-zinc-900 transition-[background,border-color,transform] duration-150 hover:scale-105 hover:border-zinc-700"
    }
}

pub(super) fn hex_input_class(valid: bool) -> &'static str {
    if valid {
        "h-11 min-w-0 flex-1 rounded-xl border border-zinc-800 bg-zinc-900 px-3 font-mono text-[13px] text-zinc-200 outline-none transition-[background,border-color,box-shadow] duration-150 placeholder:text-zinc-600 focus:border-zinc-700 focus:shadow-[0_0_0_4px_rgba(59,130,246,0.08)]"
    } else {
        "h-11 min-w-0 flex-1 rounded-xl border border-red-500/40 bg-red-950/20 px-3 font-mono text-[13px] text-red-100 outline-none transition-[background,border-color,box-shadow] duration-150 placeholder:text-red-300/40 focus:border-red-400/60 focus:shadow-[0_0_0_4px_rgba(248,113,113,0.08)]"
    }
}

pub(super) fn permission_row_class(locked: bool) -> &'static str {
    if locked {
        "flex cursor-not-allowed items-center justify-between gap-4 py-3 first:pt-4 last:pb-4"
    } else {
        "flex cursor-pointer items-center justify-between gap-4 py-3 first:pt-4 last:pb-4 transition-[background,transform] duration-150 hover:translate-x-0.5 hover:bg-zinc-900/45"
    }
}

pub(super) fn toggle_track_class(checked: bool, locked: bool) -> &'static str {
    match (checked, locked) {
        (true, true) => {
            "relative inline-block h-6 w-11 rounded-full bg-accent/80 opacity-70 transition-[background,box-shadow] duration-200"
        }
        (true, false) => {
            "relative inline-block h-6 w-11 rounded-full bg-accent shadow-[0_0_0_4px_rgba(59,130,246,0.10)] transition-[background,box-shadow] duration-200"
        }
        (false, _) => {
            "relative inline-block h-6 w-11 rounded-full bg-zinc-800 transition-[background,box-shadow] duration-200"
        }
    }
}

pub(super) fn toggle_knob_class(checked: bool) -> &'static str {
    if checked {
        "absolute left-[22px] top-1 h-4 w-4 rounded-full bg-white transition-[left,background,transform] duration-200 ease-out"
    } else {
        "absolute left-1 top-1 h-4 w-4 rounded-full bg-zinc-500 transition-[left,background,transform] duration-200 ease-out"
    }
}

pub(super) fn save_bar_class(dirty: bool) -> &'static str {
    if dirty {
        "fixed inset-x-0 bottom-0 z-50 p-4 transition-[opacity,transform] duration-200"
    } else {
        "pointer-events-none fixed inset-x-0 bottom-0 z-50 translate-y-full p-4 opacity-0 transition-[opacity,transform] duration-200"
    }
}

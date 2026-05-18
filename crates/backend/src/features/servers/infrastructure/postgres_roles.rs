//! Postgres role storage helpers.

use std::collections::HashMap;

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use uuid::Uuid;

use crate::features::servers::domain::ServerRole;
use crate::features::servers::infrastructure::entities::{
    server_member_roles, server_role_permissions, server_roles,
};
use crate::features::servers::infrastructure::postgres_conversions::{
    role_kind_as_str, role_permission_as_str, role_permission_from_str, server_role_from_model,
};

pub(super) async fn list_server_roles(
    database: &DatabaseConnection,
    server_id: &Uuid,
) -> anyhow::Result<Vec<ServerRole>> {
    let rows = server_roles::Entity::find()
        .filter(server_roles::Column::ServerId.eq(*server_id))
        .order_by_asc(server_roles::Column::Position)
        .all(database)
        .await?;
    let role_ids = rows.iter().map(|role| role.id).collect::<Vec<_>>();
    let permission_rows = if role_ids.is_empty() {
        Vec::new()
    } else {
        server_role_permissions::Entity::find()
            .filter(server_role_permissions::Column::RoleId.is_in(role_ids))
            .all(database)
            .await?
    };
    let mut permissions_by_role = HashMap::<Uuid, Vec<_>>::new();
    for permission_row in permission_rows {
        permissions_by_role
            .entry(permission_row.role_id)
            .or_default()
            .push(role_permission_from_str(&permission_row.permission)?);
    }

    rows.into_iter()
        .map(|row| {
            let permissions = permissions_by_role.remove(&row.id).unwrap_or_default();
            server_role_from_model(row, permissions)
        })
        .collect()
}

pub(super) async fn list_server_member_roles(
    database: &DatabaseConnection,
    server_id: &Uuid,
) -> anyhow::Result<Vec<(Uuid, Uuid)>> {
    let rows = server_member_roles::Entity::find()
        .filter(server_member_roles::Column::ServerId.eq(*server_id))
        .all(database)
        .await?;

    Ok(rows.into_iter().map(|row| (row.user_id, row.role_id)).collect())
}

pub(super) async fn assign_server_member_role(
    database: &DatabaseConnection,
    server_id: &Uuid,
    user_id: &Uuid,
    role_id: &Uuid,
    granted_by_user_id: &Uuid,
) -> anyhow::Result<()> {
    let existing = server_member_roles::Entity::find()
        .filter(server_member_roles::Column::ServerId.eq(*server_id))
        .filter(server_member_roles::Column::UserId.eq(*user_id))
        .filter(server_member_roles::Column::RoleId.eq(*role_id))
        .one(database)
        .await?;
    if existing.is_none() {
        server_member_roles::ActiveModel {
            server_id: Set(*server_id),
            user_id: Set(*user_id),
            role_id: Set(*role_id),
            granted_by_user_id: Set(*granted_by_user_id),
            assigned_at: Set(Utc::now()),
        }
        .insert(database)
        .await?;
    }

    Ok(())
}

pub(super) async fn revoke_server_member_role(
    database: &DatabaseConnection,
    server_id: &Uuid,
    user_id: &Uuid,
    role_id: &Uuid,
) -> anyhow::Result<()> {
    server_member_roles::Entity::delete_many()
        .filter(server_member_roles::Column::ServerId.eq(*server_id))
        .filter(server_member_roles::Column::UserId.eq(*user_id))
        .filter(server_member_roles::Column::RoleId.eq(*role_id))
        .exec(database)
        .await?;

    Ok(())
}

pub(super) async fn replace_server_roles(
    database: &DatabaseConnection,
    server_id: &Uuid,
    roles: Vec<ServerRole>,
) -> anyhow::Result<Vec<ServerRole>> {
    let transaction = database.begin().await?;
    server_roles::Entity::delete_many()
        .filter(server_roles::Column::ServerId.eq(*server_id))
        .exec(&transaction)
        .await?;

    let mut saved = Vec::with_capacity(roles.len());
    for role in roles {
        let permissions = role.permissions.clone();
        let model = server_roles::ActiveModel {
            id: Set(role.id),
            server_id: Set(role.server_id),
            name: Set(role.name),
            color: Set(role.color),
            kind: Set(role_kind_as_str(role.kind).to_owned()),
            position: Set(role.position.try_into().unwrap_or(i32::MAX)),
            created_at: Set(role.created_at),
            updated_at: Set(role.updated_at),
        }
        .insert(&transaction)
        .await?;
        for permission in &permissions {
            server_role_permissions::ActiveModel {
                role_id: Set(model.id),
                permission: Set(role_permission_as_str(*permission).to_owned()),
            }
            .insert(&transaction)
            .await?;
        }
        saved.push(server_role_from_model(model, permissions)?);
    }

    transaction.commit().await?;

    Ok(saved)
}

//! Postgres role storage helpers.

use std::collections::HashMap;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use uuid::Uuid;

use crate::features::servers::domain::ServerRole;
use crate::features::servers::infrastructure::entities::{server_role_permissions, server_roles};
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

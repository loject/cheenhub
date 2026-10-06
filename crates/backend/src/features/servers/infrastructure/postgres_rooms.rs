//! Postgres room storage helpers.

use cheenhub_contracts::rest::ServerRoomKind;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use uuid::Uuid;

use crate::features::servers::domain::{ServerRoom, ServerRoomWriteAccess};
use crate::features::servers::infrastructure::entities::{server_room_write_roles, server_rooms};
use crate::features::servers::infrastructure::postgres_conversions::{
    room_kind_as_str, server_room_from_model, write_access_mode_as_str,
};

pub(super) async fn insert_server_room(
    database: &DatabaseConnection,
    server_id: &Uuid,
    name: String,
    kind: ServerRoomKind,
    write_access: ServerRoomWriteAccess,
) -> anyhow::Result<ServerRoom> {
    let transaction = database.begin().await?;
    let position = server_rooms::Entity::find()
        .filter(server_rooms::Column::ServerId.eq(*server_id))
        .order_by_desc(server_rooms::Column::Position)
        .one(&transaction)
        .await?
        .map(|room| room.position.saturating_add(1))
        .unwrap_or(0);
    let now = Utc::now();
    let active = server_rooms::ActiveModel {
        id: Set(Uuid::new_v4()),
        server_id: Set(*server_id),
        name: Set(name),
        kind: Set(room_kind_as_str(kind).to_owned()),
        position: Set(position),
        write_access_mode: Set(write_access_mode_as_str(write_access.mode).to_owned()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let model = active.insert(&transaction).await?;
    replace_write_roles(&transaction, model.id, &write_access.role_ids).await?;
    transaction.commit().await?;

    server_room_from_model(model, write_access.role_ids)
}

pub(super) async fn list_server_rooms(
    database: &DatabaseConnection,
    server_id: &Uuid,
) -> anyhow::Result<Vec<ServerRoom>> {
    let rows = server_rooms::Entity::find()
        .filter(server_rooms::Column::ServerId.eq(*server_id))
        .order_by_asc(server_rooms::Column::Position)
        .all(database)
        .await?;
    let write_roles = write_roles_by_room(database, &rows).await?;

    rows.into_iter()
        .map(|row| {
            let role_ids = write_roles.get(&row.id).cloned().unwrap_or_default();
            server_room_from_model(row, role_ids)
        })
        .collect()
}

pub(super) async fn find_server_room(
    database: &DatabaseConnection,
    server_id: &Uuid,
    room_id: &Uuid,
) -> anyhow::Result<Option<ServerRoom>> {
    let Some(row) = server_rooms::Entity::find()
        .filter(server_rooms::Column::ServerId.eq(*server_id))
        .filter(server_rooms::Column::Id.eq(*room_id))
        .one(database)
        .await?
    else {
        return Ok(None);
    };
    let role_ids = list_room_write_roles(database, room_id).await?;

    server_room_from_model(row, role_ids).map(Some)
}

pub(super) async fn update_server_room(
    database: &DatabaseConnection,
    server_id: &Uuid,
    room_id: &Uuid,
    name: String,
    kind: ServerRoomKind,
    write_access: ServerRoomWriteAccess,
) -> anyhow::Result<Option<ServerRoom>> {
    let transaction = database.begin().await?;
    let Some(row) = server_rooms::Entity::find()
        .filter(server_rooms::Column::ServerId.eq(*server_id))
        .filter(server_rooms::Column::Id.eq(*room_id))
        .one(&transaction)
        .await?
    else {
        transaction.rollback().await?;
        return Ok(None);
    };
    let mut active = row.into_active_model();
    active.name = Set(name);
    active.kind = Set(room_kind_as_str(kind).to_owned());
    active.write_access_mode = Set(write_access_mode_as_str(write_access.mode).to_owned());
    active.updated_at = Set(Utc::now());
    let room = active.update(&transaction).await?;
    replace_write_roles(&transaction, *room_id, &write_access.role_ids).await?;
    transaction.commit().await?;

    server_room_from_model(room, write_access.role_ids).map(Some)
}

async fn replace_write_roles<C>(
    connection: &C,
    room_id: Uuid,
    role_ids: &[Uuid],
) -> anyhow::Result<()>
where
    C: sea_orm::ConnectionTrait,
{
    server_room_write_roles::Entity::delete_many()
        .filter(server_room_write_roles::Column::RoomId.eq(room_id))
        .exec(connection)
        .await?;
    for role_id in role_ids {
        server_room_write_roles::ActiveModel {
            room_id: Set(room_id),
            role_id: Set(*role_id),
        }
        .insert(connection)
        .await?;
    }

    Ok(())
}

pub(super) async fn list_room_write_roles(
    database: &DatabaseConnection,
    room_id: &Uuid,
) -> anyhow::Result<Vec<Uuid>> {
    let rows = server_room_write_roles::Entity::find()
        .filter(server_room_write_roles::Column::RoomId.eq(*room_id))
        .all(database)
        .await?;

    Ok(rows.into_iter().map(|row| row.role_id).collect())
}

async fn write_roles_by_room<C>(
    connection: &C,
    rooms: &[server_rooms::Model],
) -> anyhow::Result<std::collections::HashMap<Uuid, Vec<Uuid>>>
where
    C: sea_orm::ConnectionTrait,
{
    let room_ids: Vec<Uuid> = rooms.iter().map(|room| room.id).collect();
    let rows = server_room_write_roles::Entity::find()
        .filter(server_room_write_roles::Column::RoomId.is_in(room_ids))
        .all(connection)
        .await?;
    let mut result = std::collections::HashMap::new();
    for row in rows {
        result
            .entry(row.room_id)
            .or_insert_with(Vec::new)
            .push(row.role_id);
    }

    Ok(result)
}

pub(super) async fn delete_server_room(
    database: &DatabaseConnection,
    server_id: &Uuid,
    room_id: &Uuid,
) -> anyhow::Result<()> {
    if let Some(room) = server_rooms::Entity::find()
        .filter(server_rooms::Column::ServerId.eq(*server_id))
        .filter(server_rooms::Column::Id.eq(*room_id))
        .one(database)
        .await?
    {
        server_rooms::Entity::delete_by_id(room.id)
            .exec(database)
            .await?;
    }

    Ok(())
}

/// Считает комнаты всех серверов в PostgreSQL.
pub(super) async fn count_all_rooms(database: &DatabaseConnection) -> anyhow::Result<u64> {
    Ok(server_rooms::Entity::find().count(database).await?)
}

pub(super) async fn count_server_rooms(
    database: &DatabaseConnection,
    server_id: &Uuid,
) -> anyhow::Result<u32> {
    let count = server_rooms::Entity::find()
        .filter(server_rooms::Column::ServerId.eq(*server_id))
        .count(database)
        .await?;

    Ok(count.try_into().unwrap_or(u32::MAX))
}

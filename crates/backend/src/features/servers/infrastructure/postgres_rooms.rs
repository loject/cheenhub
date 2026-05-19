//! Postgres room storage helpers.

use cheenhub_contracts::rest::ServerRoomKind;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::features::servers::domain::ServerRoom;
use crate::features::servers::infrastructure::entities::server_rooms;
use crate::features::servers::infrastructure::postgres_conversions::{
    room_kind_as_str, server_room_from_model,
};

pub(super) async fn insert_server_room(
    database: &DatabaseConnection,
    server_id: &Uuid,
    name: String,
    kind: ServerRoomKind,
) -> anyhow::Result<ServerRoom> {
    let position = server_rooms::Entity::find()
        .filter(server_rooms::Column::ServerId.eq(*server_id))
        .order_by_desc(server_rooms::Column::Position)
        .one(database)
        .await?
        .map(|room| room.position.saturating_add(1))
        .unwrap_or(0);
    let now = Utc::now();
    let model = server_rooms::ActiveModel {
        id: Set(Uuid::new_v4()),
        server_id: Set(*server_id),
        name: Set(name),
        kind: Set(room_kind_as_str(kind).to_owned()),
        position: Set(position),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(database)
    .await?;

    server_room_from_model(model)
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

    rows.into_iter().map(server_room_from_model).collect()
}

pub(super) async fn find_server_room(
    database: &DatabaseConnection,
    server_id: &Uuid,
    room_id: &Uuid,
) -> anyhow::Result<Option<ServerRoom>> {
    server_rooms::Entity::find()
        .filter(server_rooms::Column::ServerId.eq(*server_id))
        .filter(server_rooms::Column::Id.eq(*room_id))
        .one(database)
        .await?
        .map(server_room_from_model)
        .transpose()
}

pub(super) async fn update_server_room(
    database: &DatabaseConnection,
    server_id: &Uuid,
    room_id: &Uuid,
    name: String,
    kind: ServerRoomKind,
) -> anyhow::Result<Option<ServerRoom>> {
    let Some(room) = server_rooms::Entity::find()
        .filter(server_rooms::Column::ServerId.eq(*server_id))
        .filter(server_rooms::Column::Id.eq(*room_id))
        .one(database)
        .await?
    else {
        return Ok(None);
    };
    let mut room = room.into_active_model();
    room.name = Set(name);
    room.kind = Set(room_kind_as_str(kind).to_owned());
    room.updated_at = Set(Utc::now());
    let room = room.update(database).await?;

    server_room_from_model(room).map(Some)
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

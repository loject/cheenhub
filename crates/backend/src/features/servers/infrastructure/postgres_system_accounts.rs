//! Операции с системным профилем сервера в PostgreSQL.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set, TransactionTrait,
};
use uuid::Uuid;

use crate::features::auth::infrastructure::system_account;
use crate::features::servers::domain::{Server, ServerAccess, ServerSystemAccount};
use crate::features::servers::infrastructure::entities::{server_members, servers};

/// Атомарно создаёт сервер вместе с принадлежащим ему системным аккаунтом.
pub(super) async fn insert_server(
    database: &DatabaseConnection,
    owner_user_id: &Uuid,
    name: String,
) -> anyhow::Result<Server> {
    let now = Utc::now();
    let transaction = database.begin().await?;
    let model = servers::ActiveModel {
        id: Set(Uuid::new_v4()),
        owner_user_id: Set(*owner_user_id),
        name: Set(name),
        avatar_image_id: Set(None),
        audio_bitrate_bps: Set(cheenhub_contracts::media::VOICE_AUDIO_BITRATE_BPS as i32),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&transaction)
    .await?;
    system_account::insert(
        &transaction,
        model.id,
        model.name.clone(),
        model.avatar_image_id,
        now,
    )
    .await?;
    transaction.commit().await?;
    Ok(model.into())
}

pub(super) async fn list_servers(
    database: &DatabaseConnection,
    user_id: &Uuid,
) -> anyhow::Result<Vec<ServerAccess>> {
    let mut result: Vec<ServerAccess> = servers::Entity::find()
        .filter(servers::Column::OwnerUserId.eq(*user_id))
        .all(database)
        .await?
        .into_iter()
        .map(|row| ServerAccess {
            server: row.into(),
            is_member: true,
        })
        .collect();
    let members = server_members::Entity::find()
        .filter(server_members::Column::UserId.eq(*user_id))
        .filter(server_members::Column::LeftAt.is_null())
        .all(database)
        .await?;
    let ids = members
        .into_iter()
        .map(|member| member.server_id)
        .filter(|id| !result.iter().any(|access| access.server.id == *id))
        .collect::<Vec<_>>();
    if !ids.is_empty() {
        result.extend(
            servers::Entity::find()
                .filter(servers::Column::Id.is_in(ids))
                .all(database)
                .await?
                .into_iter()
                .map(|row| ServerAccess {
                    server: row.into(),
                    is_member: true,
                }),
        );
    }
    Ok(result)
}

pub(super) async fn find_owned(
    database: &DatabaseConnection,
    server_id: &Uuid,
    owner_id: &Uuid,
) -> anyhow::Result<Option<ServerSystemAccount>> {
    if servers::Entity::find()
        .filter(servers::Column::Id.eq(*server_id))
        .filter(servers::Column::OwnerUserId.eq(*owner_id))
        .one(database)
        .await?
        .is_none()
    {
        return Ok(None);
    }
    Ok(system_account::find_profile(database, *server_id)
        .await?
        .map(|profile| ServerSystemAccount {
            id: *server_id,
            nickname: profile.nickname,
            avatar_image_id: profile.avatar_image_id,
        }))
}

pub(super) async fn find_for_server(
    database: &DatabaseConnection,
    server_id: &Uuid,
) -> anyhow::Result<Option<ServerSystemAccount>> {
    Ok(system_account::find_profile(database, *server_id)
        .await?
        .map(|profile| ServerSystemAccount {
            id: *server_id,
            nickname: profile.nickname,
            avatar_image_id: profile.avatar_image_id,
        }))
}

pub(super) async fn update_nickname(
    database: &DatabaseConnection,
    server_id: &Uuid,
    owner_id: &Uuid,
    nickname: String,
) -> anyhow::Result<Option<ServerSystemAccount>> {
    if servers::Entity::find()
        .filter(servers::Column::Id.eq(*server_id))
        .filter(servers::Column::OwnerUserId.eq(*owner_id))
        .one(database)
        .await?
        .is_none()
    {
        return Ok(None);
    }
    let transaction = database.begin().await?;
    if !system_account::update_nickname(&transaction, *server_id, nickname, Utc::now()).await? {
        return Ok(None);
    }
    transaction.commit().await?;
    find_for_server(database, server_id).await
}

pub(super) async fn update_server_avatar(
    database: &DatabaseConnection,
    server_id: &Uuid,
    owner_id: &Uuid,
    avatar_id: Uuid,
) -> anyhow::Result<Option<Server>> {
    let transaction = database.begin().await?;
    let Some(server) = servers::Entity::find()
        .filter(servers::Column::Id.eq(*server_id))
        .filter(servers::Column::OwnerUserId.eq(*owner_id))
        .one(&transaction)
        .await?
    else {
        return Ok(None);
    };
    if !system_account::update_avatar(&transaction, *server_id, avatar_id, Utc::now()).await? {
        anyhow::bail!("server {server_id} has no system account");
    }
    let mut server = server.into_active_model();
    server.avatar_image_id = Set(Some(avatar_id));
    server.updated_at = Set(Utc::now());
    let updated = server.update(&transaction).await?;
    transaction.commit().await?;
    Ok(Some(updated.into()))
}

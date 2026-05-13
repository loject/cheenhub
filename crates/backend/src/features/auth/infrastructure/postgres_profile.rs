//! Postgres user profile update helpers.

use chrono::{DateTime, Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use uuid::Uuid;

use crate::features::auth::domain::UserAccount;
use crate::features::auth::infrastructure::entities::{
    user_nickname_history, user_password_change_trace, users,
};
use crate::features::auth::infrastructure::{UpdateUserNicknameError, UserConflict};

/// Updates a user's public nickname.
pub(super) async fn update_user_nickname(
    database: &DatabaseConnection,
    user_id: &Uuid,
    session_id: &Uuid,
    nickname: String,
    now: DateTime<Utc>,
    cooldown: Duration,
) -> Result<Option<UserAccount>, UpdateUserNicknameError> {
    let transaction = database
        .begin()
        .await
        .map_err(UpdateUserNicknameError::Database)?;
    let Some(user) = users::Entity::find_by_id(*user_id)
        .one(&transaction)
        .await
        .map_err(UpdateUserNicknameError::Database)?
    else {
        return Ok(None);
    };

    let cooldown_cutoff = now - cooldown;
    let result = users::Entity::update_many()
        .col_expr(
            users::Column::Nickname,
            sea_orm::sea_query::Expr::value(nickname.clone()),
        )
        .col_expr(
            users::Column::NicknameUpdatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .col_expr(
            users::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(users::Column::Id.eq(*user_id))
        .filter(users::Column::NicknameUpdatedAt.lte(cooldown_cutoff))
        .exec(&transaction)
        .await
        .map_err(map_update_user_nickname_error)?;

    if result.rows_affected == 1 {
        if user.nickname != nickname {
            user_nickname_history::ActiveModel {
                id: Set(Uuid::new_v4()),
                user_id: Set(*user_id),
                session_id: Set(*session_id),
                old_nickname: Set(user.nickname),
                new_nickname: Set(nickname),
                changed_at: Set(now),
            }
            .insert(&transaction)
            .await
            .map_err(UpdateUserNicknameError::Database)?;
        }
        let user = users::Entity::find_by_id(*user_id)
            .one(&transaction)
            .await
            .map_err(UpdateUserNicknameError::Database)?
            .map(Into::into);
        transaction
            .commit()
            .await
            .map_err(UpdateUserNicknameError::Database)?;
        return Ok(user);
    }

    if user.nickname == nickname {
        return Ok(Some(user.into()));
    }

    Err(UpdateUserNicknameError::Cooldown {
        next_allowed_at: user.nickname_updated_at + cooldown,
    })
}

/// Updates a user's password hash and records the password change trace.
pub(super) async fn change_user_password(
    database: &DatabaseConnection,
    user_id: &Uuid,
    session_id: &Uuid,
    password_hash: String,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    let transaction = database.begin().await?;
    users::Entity::update_many()
        .col_expr(
            users::Column::PasswordHash,
            sea_orm::sea_query::Expr::value(Some(password_hash)),
        )
        .col_expr(
            users::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(users::Column::Id.eq(*user_id))
        .exec(&transaction)
        .await?;

    user_password_change_trace::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(*user_id),
        session_id: Set(*session_id),
        changed_at: Set(now),
    }
    .insert(&transaction)
    .await?;

    transaction.commit().await?;
    Ok(())
}

fn map_update_user_nickname_error(error: sea_orm::DbErr) -> UpdateUserNicknameError {
    let message = error.to_string();
    if message.contains("users_nickname_key") {
        return UpdateUserNicknameError::Conflict(UserConflict::Nickname);
    }

    UpdateUserNicknameError::Database(error)
}

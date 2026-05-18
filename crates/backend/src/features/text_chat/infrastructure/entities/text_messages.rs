//! Text message entity.

use sea_orm::entity::prelude::*;

/// Text message database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "text_messages")]
pub struct Model {
    /// Stable message identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Server the message belongs to.
    pub server_id: Uuid,
    /// Room the message belongs to.
    pub room_id: Uuid,
    /// User that authored the message.
    pub author_user_id: Uuid,
    /// Author nickname snapshot.
    pub author_nickname: String,
    /// Message body.
    pub body: String,
    /// Message creation timestamp.
    pub created_at: DateTimeUtc,
    /// Soft-delete timestamp; set when the author deletes their message.
    pub deleted_at: Option<DateTimeUtc>,
}

/// Text message relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

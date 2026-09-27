//! Сущность роли, которой разрешено писать в комнату.

use sea_orm::entity::prelude::*;

/// Строка базы данных связи «комната — роль с правом записи».
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "server_room_write_roles")]
pub struct Model {
    /// Комната, для которой настроено право записи.
    #[sea_orm(primary_key, auto_increment = false)]
    pub room_id: Uuid,
    /// Роль, которой разрешено писать в комнату.
    #[sea_orm(primary_key, auto_increment = false)]
    pub role_id: Uuid,
}

/// Связи ролей, которым разрешено писать в комнату.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

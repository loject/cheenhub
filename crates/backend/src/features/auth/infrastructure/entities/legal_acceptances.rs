//! SeaORM-сущность журнала подтверждений юридических документов.

use sea_orm::entity::prelude::*;

/// Строка подтверждения конкретной версии юридического документа.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "legal_acceptances")]
pub struct Model {
    /// Стабильный идентификатор подтверждения.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Идентификатор пользователя, давшего подтверждение.
    pub user_id: Uuid,
    /// Стабильный вид подтверждённого документа.
    pub document_kind: String,
    /// Версия документа на момент подтверждения.
    pub document_version: String,
    /// Регистрационный поток, в котором получено подтверждение.
    pub acceptance_source: String,
    /// Время получения подтверждения.
    pub accepted_at: DateTimeUtc,
}

/// Связи журнала подтверждений не используются в запросах auth-инфраструктуры.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

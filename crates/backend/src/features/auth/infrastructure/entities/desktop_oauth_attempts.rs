//! Попытки desktop OAuth с явными внешними ключами и без открытых секретов.

use sea_orm::entity::prelude::*;

/// Строка попытки входа через системный браузер.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "desktop_oauth_attempts")]
pub struct Model {
    /// Идентификатор попытки.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Связанное состояние OAuth.
    pub oauth_state_id: Uuid,
    /// Хеш секрета исходного приложения и кода handoff.
    pub secret_hash: String,
    /// Срок действия попытки.
    pub expires_at: DateTimeUtc,
    /// Состояние жизненного цикла попытки.
    pub status: String,
    /// Связанный одноразовый результат.
    pub handoff_id: Option<Uuid>,
    /// Проверенный идентификатор Google.
    pub provider_subject: Option<String>,
    /// Проверенный email Google.
    pub email: Option<String>,
    /// Отображаемое имя Google.
    pub display_name: Option<String>,
    /// Безопасное сообщение об ошибке входа.
    pub error_message: Option<String>,
}

/// Явные связи попытки с состоянием и результатом OAuth.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Исходное состояние OAuth.
    #[sea_orm(
        belongs_to = "super::oauth_states::Entity",
        from = "Column::OauthStateId",
        to = "super::oauth_states::Column::Id"
    )]
    OAuthState,
    /// Одноразовый handoff для исходного приложения.
    #[sea_orm(
        belongs_to = "super::oauth_handoffs::Entity",
        from = "Column::HandoffId",
        to = "super::oauth_handoffs::Column::Id"
    )]
    Handoff,
}

impl ActiveModelBehavior for ActiveModel {}

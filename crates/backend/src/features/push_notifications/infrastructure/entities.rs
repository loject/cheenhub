//! SeaORM-сущности хранилища системных push-уведомлений.

use sea_orm::entity::prelude::*;

/// Сущность зарегистрированной push-установки.
pub(crate) mod installations {
    use sea_orm::entity::prelude::*;

    /// Строка зарегистрированной push-установки.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "push_installations")]
    pub struct Model {
        /// Стабильный идентификатор установки.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Владелец установки.
        pub user_id: Uuid,
        /// Auth-сессия, в которой зарегистрирована установка.
        pub session_id: Uuid,
        /// Имя платформы.
        pub platform: String,
        /// Непрозрачный токен push-провайдера.
        pub token: String,
        /// Признак доступности установки для новых доставок.
        pub active: bool,
        /// Время первой регистрации.
        pub created_at: DateTimeUtc,
        /// Время последнего обновления.
        pub updated_at: DateTimeUtc,
    }

    /// Отношения установки не используются напрямую.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// Сущность задания постоянной очереди.
pub(crate) mod delivery_queue {
    use sea_orm::entity::prelude::*;

    /// Строка задания доставки.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "push_delivery_queue")]
    pub struct Model {
        /// Идентификатор задания.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Целевая установка.
        pub installation_id: Uuid,
        /// Идентификатор личного сообщения.
        pub message_id: Uuid,
        /// Сериализованный data payload.
        pub payload: serde_json::Value,
        /// Число выполненных попыток.
        pub attempts: i32,
        /// Время следующей попытки.
        pub next_attempt_at: DateTimeUtc,
        /// Время постановки в очередь.
        pub created_at: DateTimeUtc,
    }

    /// Отношения задания не используются напрямую.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// Импорт нужен derive-макросам вложенных SeaORM-сущностей.
const _: Option<Uuid> = None;

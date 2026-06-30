//! SeaORM-сущности social-таблиц.

use sea_orm::entity::prelude::*;

/// Строка дружбы или заявки.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "friendships")]
pub struct Model {
    /// Стабильный идентификатор записи.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Отправитель текущей заявки.
    pub requester_user_id: Uuid,
    /// Получатель текущей заявки.
    pub recipient_user_id: Uuid,
    /// Меньший идентификатор пары.
    pub user_low_id: Uuid,
    /// Больший идентификатор пары.
    pub user_high_id: Uuid,
    /// Статус записи.
    pub status: String,
    /// Время создания.
    pub created_at: DateTimeUtc,
    /// Время последнего изменения.
    pub updated_at: DateTimeUtc,
}

/// Отношения записи дружбы.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// SeaORM-сущность личных диалогов.
pub mod dm_conversations {
    use sea_orm::entity::prelude::*;

    /// Строка личного диалога.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "dm_conversations")]
    pub struct Model {
        /// Стабильный идентификатор диалога.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Меньший идентификатор пользователя пары.
        pub user_low_id: Uuid,
        /// Больший идентификатор пользователя пары.
        pub user_high_id: Uuid,
        /// Время создания.
        pub created_at: DateTimeUtc,
        /// Время последнего изменения.
        pub updated_at: DateTimeUtc,
    }

    /// Отношения личного диалога.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// SeaORM-сущность сообщений личных диалогов.
pub mod dm_messages {
    use sea_orm::entity::prelude::*;

    /// Строка сообщения личного диалога.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "dm_messages")]
    pub struct Model {
        /// Стабильный идентификатор сообщения.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Диалог, которому принадлежит сообщение.
        pub conversation_id: Uuid,
        /// Отправитель сообщения.
        pub sender_user_id: Uuid,
        /// Текст сообщения.
        pub body: String,
        /// Время создания.
        pub created_at: DateTimeUtc,
        /// Время последнего изменения.
        pub updated_at: DateTimeUtc,
        /// Время мягкого удаления.
        pub deleted_at: Option<DateTimeUtc>,
    }

    /// Отношения сообщения личного диалога.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

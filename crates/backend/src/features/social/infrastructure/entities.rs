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
        /// Монотонный порядковый номер внутри диалога.
        pub seq: i64,
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

/// SeaORM-сущность текущего read-state участника личного диалога.
pub mod conversation_member_states {
    use sea_orm::entity::prelude::*;

    /// Строка текущего read-state участника личного диалога.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "conversation_member_state")]
    pub struct Model {
        /// Диалог, к которому относится состояние.
        #[sea_orm(primary_key, auto_increment = false)]
        pub conversation_id: Uuid,
        /// Пользователь-участник диалога.
        #[sea_orm(primary_key, auto_increment = false)]
        pub user_id: Uuid,
        /// Последнее прочитанное сообщение.
        pub last_read_message_id: Option<Uuid>,
        /// Последний прочитанный порядковый номер.
        pub last_read_seq: i64,
        /// Серверное время последнего подтверждения прочтения.
        pub last_read_at: Option<DateTimeUtc>,
        /// Количество непрочитанных входящих сообщений.
        pub unread_count: i64,
        /// Время последнего обновления состояния.
        pub updated_at: DateTimeUtc,
    }

    /// Отношения read-state участника.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// SeaORM-сущность исторических checkpoint прочтения.
pub mod conversation_read_checkpoints {
    use sea_orm::entity::prelude::*;

    /// Строка исторического checkpoint прочтения.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "conversation_read_checkpoints")]
    pub struct Model {
        /// Стабильный идентификатор checkpoint.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Диалог, к которому относится checkpoint.
        pub conversation_id: Uuid,
        /// Пользователь, который прочитал сообщения.
        pub user_id: Uuid,
        /// Последнее прочитанное сообщение.
        pub last_read_message_id: Uuid,
        /// Последний прочитанный порядковый номер.
        pub last_read_seq: i64,
        /// Серверное время подтверждения прочтения.
        pub read_at: DateTimeUtc,
        /// Время создания записи.
        pub created_at: DateTimeUtc,
    }

    /// Отношения checkpoint прочтения.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

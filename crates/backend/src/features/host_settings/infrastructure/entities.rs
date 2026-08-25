//! SeaORM-сущности глобальных настроек хоста.

use sea_orm::entity::prelude::*;

/// Сущность владельца конкретной установки CheenHub.
pub(crate) mod host_owners {
    use sea_orm::entity::prelude::*;

    /// Пользователь с глобальными правами владельца хоста.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "host_owners")]
    pub struct Model {
        /// Идентификатор пользователя и первичный ключ записи.
        #[sea_orm(primary_key, auto_increment = false)]
        pub user_id: Uuid,
        /// Время выдачи глобальных прав.
        pub granted_at: DateTimeUtc,
        /// Пользователь, выдавший глобальные права.
        pub granted_by_user_id: Option<Uuid>,
    }

    /// Связи владельца хоста не используются напрямую.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// Singleton-сущность настройки исходящей почты хоста.
pub(crate) mod host_email_settings {
    use sea_orm::entity::prelude::*;

    /// Единственная строка конфигурации транспорта исходящей почты.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "host_email_settings")]
    pub struct Model {
        /// Фиксированный нулевой UUID singleton-записи.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Выбранный транспорт: `smtp` или `gmail_api`.
        pub transport: String,
        /// Общий таймаут одной попытки отправки в секундах.
        pub email_send_timeout_seconds: i32,
        /// Адрес SMTP-сервера.
        pub smtp_host: Option<String>,
        /// TCP-порт SMTP-сервера.
        pub smtp_port: Option<i32>,
        /// Имя пользователя SMTP.
        pub smtp_username: Option<String>,
        /// Пароль SMTP в открытом виде.
        pub smtp_password: Option<String>,
        /// Адрес отправителя для SMTP.
        pub smtp_from_email: Option<String>,
        /// OAuth Client ID приложения Gmail.
        pub gmail_client_id: Option<String>,
        /// OAuth Client Secret приложения Gmail в открытом виде.
        pub gmail_client_secret: Option<String>,
        /// OAuth Refresh Token Gmail в открытом виде.
        pub gmail_refresh_token: Option<String>,
        /// Адрес отправителя Gmail.
        pub gmail_from_email: Option<String>,
        /// Время последнего обновления настройки.
        pub updated_at: DateTimeUtc,
        /// Пользователь, последним обновивший настройку.
        pub updated_by_user_id: Option<Uuid>,
    }

    /// Связи настройки почты не используются напрямую.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// Сущность одноразового состояния OAuth-подключения Gmail.
pub(crate) mod host_gmail_oauth_states {
    use sea_orm::entity::prelude::*;

    /// Короткоживущая запись проверки OAuth callback.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "host_gmail_oauth_states")]
    pub struct Model {
        /// Стабильный идентификатор состояния.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// SHA-256 hash непрозрачного значения `state`.
        pub state_hash: String,
        /// Владелец хоста, запустивший подключение.
        pub user_id: Uuid,
        /// Время создания состояния.
        pub created_at: DateTimeUtc,
        /// Время истечения состояния.
        pub expires_at: DateTimeUtc,
        /// Время одноразового потребления состояния.
        pub consumed_at: Option<DateTimeUtc>,
    }

    /// Связи OAuth-состояния не используются напрямую.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// Импорт нужен derive-макросам вложенных SeaORM-сущностей.
const _: Option<Uuid> = None;

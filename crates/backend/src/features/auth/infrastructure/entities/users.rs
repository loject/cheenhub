//! User account entity.

use sea_orm::entity::prelude::*;

/// User account database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    /// Стабильный идентификатор пользователя.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Публичное имя, показываемое другим пользователям.
    pub nickname: String,
    /// Категория аккаунта, определяющая доступные учётные данные и публичные операции.
    pub kind: String,
    /// Адрес для входа; отсутствует у видов учётных записей без входа.
    pub email: Option<String>,
    /// Нормализованный адрес для поиска и уникальности; отсутствует у видов без входа.
    pub email_normalized: Option<String>,
    /// Сохранённый хеш пароля Argon2.
    pub password_hash: Option<String>,
    /// Идентификатор текущего изображения аватара.
    pub avatar_image_id: Option<Uuid>,
    /// Время регистрации учётной записи.
    pub registered_at: DateTimeUtc,
    /// Время последнего успешного изменения имени.
    pub nickname_updated_at: DateTimeUtc,
    /// Время запроса удаления аккаунта.
    pub deletion_requested_at: Option<DateTimeUtc>,
    /// Исключительная граница срока восстановления аккаунта.
    pub deletion_restore_until: Option<DateTimeUtc>,
    /// Хеш одноразового секрета восстановления.
    pub deletion_token_hash: Option<String>,
    /// Время окончательного обезличивания аккаунта.
    pub deletion_finalized_at: Option<DateTimeUtc>,
    /// Last account update timestamp.
    pub updated_at: DateTimeUtc,
}

/// User account relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

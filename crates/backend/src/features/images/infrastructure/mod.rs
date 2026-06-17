//! Инфраструктурный слой изображений.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use std::sync::Mutex;
use uuid::Uuid;

use crate::features::images::domain::{NewStoredImage, StoredImage};

mod entities {
    pub(crate) mod images;
}

/// Граница хранилища изображений.
#[async_trait]
pub(crate) trait ImageStore: Send + Sync {
    /// Вставляет обработанное изображение.
    async fn insert_image(&self, image: NewStoredImage) -> anyhow::Result<()>;

    /// Находит сохраненное изображение по идентификатору.
    async fn find_image(&self, image_id: &Uuid) -> anyhow::Result<Option<StoredImage>>;
}

/// Хранилище изображений на базе Postgres.
pub(crate) struct PostgresImageStore {
    database: DatabaseConnection,
}

impl PostgresImageStore {
    /// Создает хранилище изображений на базе Postgres.
    pub(crate) fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }
}

#[async_trait]
impl ImageStore for PostgresImageStore {
    async fn insert_image(&self, image: NewStoredImage) -> anyhow::Result<()> {
        let now = chrono::Utc::now();
        entities::images::ActiveModel {
            id: Set(image.id),
            owner_user_id: Set(image.owner_user_id),
            kind: Set(image.kind),
            content_type: Set(image.content_type),
            width: Set(image.width),
            height: Set(image.height),
            byte_size: Set(image.byte_size),
            sha256: Set(image.sha256),
            storage_backend: Set(image.storage_backend),
            storage_key: Set(image.storage_key),
            data: Set(image.data),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&self.database)
        .await?;

        Ok(())
    }

    async fn find_image(&self, image_id: &Uuid) -> anyhow::Result<Option<StoredImage>> {
        Ok(entities::images::Entity::find_by_id(*image_id)
            .one(&self.database)
            .await?
            .map(Into::into))
    }
}

/// In-memory-хранилище изображений для тестов и локальной разработки.
#[derive(Default)]
pub(crate) struct InMemoryImageStore {
    images: Mutex<Vec<StoredImage>>,
}

#[async_trait]
impl ImageStore for InMemoryImageStore {
    async fn insert_image(&self, image: NewStoredImage) -> anyhow::Result<()> {
        self.images
            .lock()
            .map_err(|_| anyhow::anyhow!("in-memory image store lock poisoned"))?
            .push(StoredImage {
                id: image.id,
                owner_user_id: image.owner_user_id,
                kind: image.kind,
                content_type: image.content_type,
                width: image.width,
                height: image.height,
                byte_size: image.byte_size,
                storage_backend: image.storage_backend,
                data: image.data,
            });

        Ok(())
    }

    async fn find_image(&self, image_id: &Uuid) -> anyhow::Result<Option<StoredImage>> {
        Ok(self
            .images
            .lock()
            .map_err(|_| anyhow::anyhow!("in-memory image store lock poisoned"))?
            .iter()
            .find(|image| image.id == *image_id)
            .cloned())
    }
}

impl From<entities::images::Model> for StoredImage {
    fn from(row: entities::images::Model) -> Self {
        Self {
            id: row.id,
            owner_user_id: row.owner_user_id,
            kind: row.kind,
            content_type: row.content_type,
            width: row.width,
            height: row.height,
            byte_size: row.byte_size,
            storage_backend: row.storage_backend,
            data: row.data,
        }
    }
}

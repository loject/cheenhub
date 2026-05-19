//! Object storage for text chat attachment bytes.

use async_trait::async_trait;
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::Builder as S3ConfigBuilder;
use aws_sdk_s3::primitives::ByteStream;
#[cfg(test)]
use std::{collections::HashMap, sync::Mutex};

use crate::config::S3Config;

/// Stored object bytes and metadata.
pub(crate) struct StoredObject {
    /// Object bytes.
    pub(crate) bytes: Vec<u8>,
    /// Object content type.
    pub(crate) content_type: String,
}

/// Object storage boundary for text chat attachment bytes.
#[async_trait]
pub(crate) trait ChatAttachmentObjectStore: Send + Sync {
    /// Returns the configured object bucket.
    fn bucket(&self) -> Option<&str>;

    /// Writes one object.
    async fn put_object(&self, key: &str, content_type: &str, bytes: Vec<u8>)
    -> anyhow::Result<()>;

    /// Reads one object.
    async fn get_object(&self, key: &str) -> anyhow::Result<StoredObject>;
}

/// S3-compatible object storage for text chat attachment bytes.
pub(crate) struct S3ChatAttachmentObjectStore {
    client: Client,
    bucket: String,
}

impl S3ChatAttachmentObjectStore {
    /// Builds an S3-compatible object storage client.
    pub(crate) async fn from_config(config: &S3Config) -> Self {
        let credentials = Credentials::new(
            config.access_key_id.clone(),
            config.secret_access_key.clone(),
            None,
            None,
            "chat-images-s3",
        );
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .endpoint_url(config.endpoint.clone())
            .credentials_provider(credentials)
            .load()
            .await;
        let client = Client::from_conf(
            S3ConfigBuilder::from(&sdk_config)
                .force_path_style(config.force_path_style)
                .build(),
        );

        Self {
            client,
            bucket: config.bucket.clone(),
        }
    }
}

#[async_trait]
impl ChatAttachmentObjectStore for S3ChatAttachmentObjectStore {
    fn bucket(&self) -> Option<&str> {
        Some(&self.bucket)
    }

    async fn put_object(
        &self,
        key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> anyhow::Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(ByteStream::from(bytes))
            .send()
            .await?;

        Ok(())
    }

    async fn get_object(&self, key: &str) -> anyhow::Result<StoredObject> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        let content_type = output
            .content_type
            .unwrap_or_else(|| "application/octet-stream".to_owned());
        let bytes = output.body.collect().await?.into_bytes().to_vec();

        Ok(StoredObject {
            bytes,
            content_type,
        })
    }
}

/// Disabled object storage used when chat image S3 env is not configured.
#[derive(Default)]
pub(crate) struct DisabledChatAttachmentObjectStore;

#[async_trait]
impl ChatAttachmentObjectStore for DisabledChatAttachmentObjectStore {
    fn bucket(&self) -> Option<&str> {
        None
    }

    async fn put_object(
        &self,
        _key: &str,
        _content_type: &str,
        _bytes: Vec<u8>,
    ) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("chat image S3 storage is not configured"))
    }

    async fn get_object(&self, _key: &str) -> anyhow::Result<StoredObject> {
        Err(anyhow::anyhow!("chat image S3 storage is not configured"))
    }
}

/// In-memory object storage for local tests.
#[derive(Default)]
#[cfg(test)]
pub(crate) struct InMemoryChatAttachmentObjectStore {
    objects: Mutex<HashMap<String, StoredObject>>,
    bucket: String,
}

#[cfg(test)]
impl InMemoryChatAttachmentObjectStore {
    /// Builds an in-memory object storage with a bucket name.
    pub(crate) fn new(bucket: impl Into<String>) -> Self {
        Self {
            objects: Mutex::new(HashMap::new()),
            bucket: bucket.into(),
        }
    }
}

#[async_trait]
#[cfg(test)]
impl ChatAttachmentObjectStore for InMemoryChatAttachmentObjectStore {
    fn bucket(&self) -> Option<&str> {
        Some(&self.bucket)
    }

    async fn put_object(
        &self,
        key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> anyhow::Result<()> {
        self.objects
            .lock()
            .map_err(|_| anyhow::anyhow!("in-memory chat attachment object store lock poisoned"))?
            .insert(
                key.to_owned(),
                StoredObject {
                    bytes,
                    content_type: content_type.to_owned(),
                },
            );

        Ok(())
    }

    async fn get_object(&self, key: &str) -> anyhow::Result<StoredObject> {
        let object = self
            .objects
            .lock()
            .map_err(|_| anyhow::anyhow!("in-memory chat attachment object store lock poisoned"))?
            .get(key)
            .map(|object| StoredObject {
                bytes: object.bytes.clone(),
                content_type: object.content_type.clone(),
            })
            .ok_or_else(|| anyhow::anyhow!("chat attachment object was not found"))?;

        Ok(object)
    }
}

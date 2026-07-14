#![warn(missing_docs)]
//! Точка входа бэкенда CheenHub.

mod config;
mod db;
mod features;
mod http;
mod realtime;
mod state;
mod telemetry;

use anyhow::Context;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

type Stores = (
    Arc<dyn features::auth::infrastructure::AuthStore>,
    Arc<dyn features::servers::infrastructure::ServerStore>,
    Arc<dyn features::social::infrastructure::SocialStore>,
    Arc<dyn features::text_chat::infrastructure::TextChatStore>,
    Arc<dyn features::text_chat::infrastructure::ChatAttachmentObjectStore>,
    Arc<dyn features::images::infrastructure::ImageStore>,
    Arc<features::push_notifications::application::PushNotifications>,
);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = config::AppConfig::from_env()?;
    telemetry::init(&config.log_filter)?;

    let address = config.socket_addr()?;
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("failed to bind backend listener at {address}"))?;
    let auth_keys = features::auth::security::keys::AuthKeys::from_config(
        &config.jwt_private_key_base64,
        config.jwt_key_id.clone(),
    )?;
    let chat_attachment_object_store: Arc<
        dyn features::text_chat::infrastructure::ChatAttachmentObjectStore,
    > = match &config.chat_images_s3 {
        Some(s3_config) => {
            tracing::info!(
                bucket = %s3_config.bucket,
                endpoint = %s3_config.endpoint,
                region = %s3_config.region,
                force_path_style = s3_config.force_path_style,
                "configured chat image S3 storage"
            );
            Arc::new(
                features::text_chat::infrastructure::S3ChatAttachmentObjectStore::from_config(
                    s3_config,
                )
                .await,
            )
        }
        None => {
            tracing::warn!(
                missing_env = ?[
                    "CHAT_IMAGES_S3_ENDPOINT",
                    "CHAT_IMAGES_S3_REGION",
                    "CHAT_IMAGES_S3_BUCKET",
                    "CHAT_IMAGES_S3_ACCESS_KEY_ID",
                    "CHAT_IMAGES_S3_SECRET_ACCESS_KEY",
                ],
                "chat image uploads are disabled until S3 storage is configured"
            );
            Arc::new(features::text_chat::infrastructure::DisabledChatAttachmentObjectStore)
        }
    };
    let (
        auth_store,
        server_store,
        social_store,
        text_chat_store,
        chat_attachment_object_store,
        image_store,
        push_notifications,
    ): Stores = match config.auth_store {
        config::AuthStoreConfig::Postgres => {
            let database = db::connect(&config.database_url).await?;
            let auth_store: Arc<dyn features::auth::infrastructure::AuthStore> = Arc::new(
                features::auth::infrastructure::PostgresAuthStore::new(database.clone()),
            );
            let fcm = match config.fcm_service_account_path.as_deref() {
                Some(path) => {
                    tracing::info!(credential_path = %path, "configured FCM HTTP v1 delivery");
                    Some(
                        features::push_notifications::FcmClient::from_service_account_file(
                            std::path::Path::new(path),
                        )?,
                    )
                }
                None => {
                    tracing::warn!(
                        missing_env = "FCM_SERVICE_ACCOUNT_PATH",
                        "FCM delivery is disabled; push registrations and queued deliveries remain available"
                    );
                    None
                }
            };
            let push_notifications = Arc::new(
                features::push_notifications::application::PushNotifications::postgres(
                    features::push_notifications::infrastructure::PostgresPushStore::new(
                        database.clone(),
                    ),
                    fcm,
                    auth_store.clone(),
                ),
            );
            (
                auth_store,
                Arc::new(features::servers::infrastructure::PostgresServerStore::new(
                    database.clone(),
                )),
                Arc::new(features::social::infrastructure::PostgresSocialStore::new(
                    database.clone(),
                )),
                Arc::new(
                    features::text_chat::infrastructure::PostgresTextChatStore::new(
                        database.clone(),
                    ),
                ),
                chat_attachment_object_store.clone(),
                Arc::new(features::images::infrastructure::PostgresImageStore::new(
                    database,
                )),
                push_notifications,
            )
        }
        config::AuthStoreConfig::InMemory => {
            let auth_store: Arc<dyn features::auth::infrastructure::AuthStore> =
                Arc::new(features::auth::infrastructure::InMemoryAuthStore::default());
            let push_notifications = Arc::new(
                features::push_notifications::application::PushNotifications::disabled(
                    auth_store.clone(),
                ),
            );
            (
                auth_store,
                Arc::new(features::servers::infrastructure::InMemoryServerStore::default()),
                Arc::new(features::social::infrastructure::InMemorySocialStore::default()),
                Arc::new(features::text_chat::infrastructure::InMemoryTextChatStore::default()),
                chat_attachment_object_store,
                Arc::new(features::images::infrastructure::InMemoryImageStore::default()),
                push_notifications,
            )
        }
    };
    let realtime_tls = realtime::ensure_tls_config(
        config.webtransport_tls_cert_path.as_deref(),
        config.webtransport_tls_key_path.as_deref(),
    )?;

    let state = state::AppState {
        auth_store,
        auth_mailer: Arc::new(features::auth::email::SmtpAuthMailer::new(
            config.smtp_host.clone(),
            config.smtp_port,
            config.smtp_username.clone(),
            config.smtp_password.clone(),
            config.smtp_from_email.clone(),
        )?),
        server_store,
        social_store,
        text_chat_store,
        chat_attachment_object_store,
        image_store,
        push_notifications: push_notifications.clone(),
        image_processing_queue: Arc::new(tokio::sync::Semaphore::new(1)),
        voice_presence_store: Arc::new(
            features::voice_chat::infrastructure::InMemoryVoicePresenceStore::default(),
        ),
        realtime_hub: Arc::new(realtime::hub::RealtimeHub::default()),
        auth_keys,
        access_token_lifetime_minutes: config.access_token_lifetime_minutes,
        refresh_token_lifetime_days: config.refresh_token_lifetime_days,
        google_oauth_client_id: config.google_oauth_client_id.clone(),
        google_oauth_client_secret: config.google_oauth_client_secret.clone(),
        google_oauth_redirect_uri: config.google_oauth_redirect_uri.clone(),
        cheenhub_client_base_url: config.cheenhub_client_base_url.clone(),
        cheenhub_api_base_url: config.cheenhub_api_base_url.clone(),
        oauth_state_lifetime_minutes: config.oauth_state_lifetime_minutes,
        oauth_handoff_lifetime_minutes: config.oauth_handoff_lifetime_minutes,
        oauth_registration_lifetime_minutes: config.oauth_registration_lifetime_minutes,
        password_reset_token_lifetime_minutes: config.password_reset_token_lifetime_minutes,
    };
    let app = http::router(state.clone());
    if push_notifications.worker_enabled() {
        tokio::spawn(push_notifications.run_delivery_worker());
    }
    let realtime_address = config.webtransport_socket_addr()?;
    let realtime_cert_path = realtime_tls.cert_path;
    let realtime_key_path = realtime_tls.key_path;
    let realtime_server =
        realtime::bind(realtime_address, &realtime_cert_path, &realtime_key_path)?;

    tokio::spawn(async move {
        if let Err(error) = realtime::serve(state, realtime_address, realtime_server).await {
            tracing::error!(%error, "webtransport realtime listener stopped");
        }
    });

    info!(%address, "backend listening");
    axum::serve(listener, app)
        .await
        .context("backend server stopped with an error")
}

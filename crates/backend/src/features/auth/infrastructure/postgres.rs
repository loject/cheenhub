//! Postgres-backed authentication storage.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use uuid::Uuid;

use crate::features::auth::domain::{
    OAuthAccount, OAuthHandoff, OAuthRegistrationIntent, OAuthState, RefreshSession, UserAccount,
};
use crate::features::auth::infrastructure::entities::{
    oauth_accounts, oauth_handoffs, oauth_registration_intents, oauth_states, refresh_tokens,
    sessions, users,
};
use crate::features::auth::infrastructure::{AuthStore, InsertUserError, UserConflict};

/// Postgres-backed authentication storage.
pub(crate) struct PostgresAuthStore {
    database: DatabaseConnection,
}

impl PostgresAuthStore {
    /// Builds a Postgres-backed authentication storage.
    pub(crate) fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }
}

#[async_trait]
impl AuthStore for PostgresAuthStore {
    async fn insert_user(
        &self,
        nickname: String,
        email: String,
        email_normalized: String,
        password_hash: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<UserAccount, InsertUserError> {
        let user_id = Uuid::new_v4();
        let model = users::ActiveModel {
            id: Set(user_id),
            nickname: Set(nickname),
            email: Set(email),
            email_normalized: Set(email_normalized),
            password_hash: Set(password_hash),
            registered_at: Set(now),
            accepted_terms_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&self.database)
        .await
        .map_err(map_insert_user_error)?;

        Ok(model.into())
    }

    async fn find_user_by_email(
        &self,
        email_normalized: &str,
    ) -> anyhow::Result<Option<UserAccount>> {
        Ok(users::Entity::find()
            .filter(users::Column::EmailNormalized.eq(email_normalized))
            .one(&self.database)
            .await?
            .map(Into::into))
    }

    async fn find_user_by_id(&self, user_id: &Uuid) -> anyhow::Result<Option<UserAccount>> {
        Ok(users::Entity::find_by_id(*user_id)
            .one(&self.database)
            .await?
            .map(Into::into))
    }

    async fn update_user_password_hash(
        &self,
        user_id: &Uuid,
        password_hash: String,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::postgres_password_reset::update_user_password_hash(
            &self.database,
            user_id,
            password_hash,
            now,
        )
        .await
    }

    async fn create_session(
        &self,
        user_id: &Uuid,
        refresh_hash: String,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<Uuid> {
        let session_id = Uuid::new_v4();
        let refresh_id = Uuid::new_v4();

        sessions::ActiveModel {
            id: Set(session_id),
            user_id: Set(*user_id),
            created_at: Set(now),
            last_seen_at: Set(now),
            expires_at: Set(expires_at),
            revoked_at: Set(None),
        }
        .insert(&self.database)
        .await?;

        refresh_tokens::ActiveModel {
            id: Set(refresh_id),
            session_id: Set(session_id),
            token_hash: Set(refresh_hash),
            created_at: Set(now),
            rotated_at: Set(None),
            expires_at: Set(expires_at),
            revoked_at: Set(None),
        }
        .insert(&self.database)
        .await?;

        Ok(session_id)
    }

    async fn find_active_refresh(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<RefreshSession>> {
        super::postgres_refresh::find_active_refresh(&self.database, token_hash, now).await
    }

    async fn rotate_refresh(
        &self,
        old_refresh_id: &Uuid,
        session_id: &Uuid,
        next_hash: String,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        if let Some(old_refresh) = refresh_tokens::Entity::find_by_id(old_refresh_id.to_owned())
            .one(&self.database)
            .await?
        {
            let mut old_refresh = old_refresh.into_active_model();
            old_refresh.rotated_at = Set(Some(now));
            old_refresh.revoked_at = Set(Some(now));
            old_refresh.update(&self.database).await?;
        }

        if let Some(session) = sessions::Entity::find_by_id(session_id.to_owned())
            .filter(sessions::Column::RevokedAt.is_null())
            .one(&self.database)
            .await?
        {
            let mut session = session.into_active_model();
            session.last_seen_at = Set(now);
            session.expires_at = Set(expires_at);
            session.update(&self.database).await?;
        }

        refresh_tokens::ActiveModel {
            id: Set(Uuid::new_v4()),
            session_id: Set(*session_id),
            token_hash: Set(next_hash),
            created_at: Set(now),
            rotated_at: Set(None),
            expires_at: Set(expires_at),
            revoked_at: Set(None),
        }
        .insert(&self.database)
        .await?;

        Ok(())
    }

    async fn revoke_refresh_session(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let Some(refresh_token) = refresh_tokens::Entity::find()
            .filter(refresh_tokens::Column::TokenHash.eq(token_hash))
            .one(&self.database)
            .await?
        else {
            return Ok(());
        };
        let session_id = refresh_token.session_id;

        if refresh_token.revoked_at.is_none() {
            let mut refresh_token = refresh_token.into_active_model();
            refresh_token.revoked_at = Set(Some(now));
            refresh_token.update(&self.database).await?;
        }

        if let Some(session) = sessions::Entity::find_by_id(session_id)
            .filter(sessions::Column::RevokedAt.is_null())
            .one(&self.database)
            .await?
        {
            let mut session = session.into_active_model();
            session.revoked_at = Set(Some(now));
            session.update(&self.database).await?;
        }

        Ok(())
    }

    async fn session_is_active(
        &self,
        session_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        super::postgres_refresh::session_is_active(&self.database, session_id, now).await
    }

    async fn revoke_user_sessions(&self, user_id: &Uuid, now: DateTime<Utc>) -> anyhow::Result<()> {
        super::postgres_password_reset::revoke_user_sessions(&self.database, user_id, now).await
    }

    async fn insert_password_reset_token(
        &self,
        user_id: &Uuid,
        token_hash: String,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::postgres_password_reset::insert_password_reset_token(
            &self.database,
            user_id,
            token_hash,
            now,
            expires_at,
        )
        .await
    }

    async fn consume_password_reset_token(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<crate::features::auth::domain::PasswordResetToken>> {
        super::postgres_password_reset::consume_password_reset_token(
            &self.database,
            token_hash,
            now,
        )
        .await
    }

    async fn insert_oauth_state(
        &self,
        state_hash: String,
        nonce: String,
        flow_kind: String,
        user_id: Option<Uuid>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        oauth_states::ActiveModel {
            id: Set(Uuid::new_v4()),
            state_hash: Set(state_hash),
            nonce: Set(nonce),
            flow_kind: Set(flow_kind),
            user_id: Set(user_id),
            created_at: Set(now),
            expires_at: Set(expires_at),
            consumed_at: Set(None),
        }
        .insert(&self.database)
        .await?;

        Ok(())
    }

    async fn consume_oauth_state(
        &self,
        state_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthState>> {
        let Some(state) = oauth_states::Entity::find()
            .filter(oauth_states::Column::StateHash.eq(state_hash))
            .filter(oauth_states::Column::ConsumedAt.is_null())
            .filter(oauth_states::Column::ExpiresAt.gt(now))
            .one(&self.database)
            .await?
        else {
            return Ok(None);
        };

        let result = OAuthState {
            nonce: state.nonce.clone(),
            flow_kind: state.flow_kind.clone(),
            user_id: state.user_id,
        };
        let mut active = state.into_active_model();
        active.consumed_at = Set(Some(now));
        active.update(&self.database).await?;

        Ok(Some(result))
    }

    async fn find_oauth_account_by_subject(
        &self,
        provider: &str,
        provider_subject: &str,
    ) -> anyhow::Result<Option<OAuthAccount>> {
        Ok(oauth_accounts::Entity::find()
            .filter(oauth_accounts::Column::Provider.eq(provider))
            .filter(oauth_accounts::Column::ProviderSubject.eq(provider_subject))
            .one(&self.database)
            .await?
            .map(Into::into))
    }

    async fn find_oauth_account_for_user(
        &self,
        provider: &str,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<OAuthAccount>> {
        Ok(oauth_accounts::Entity::find()
            .filter(oauth_accounts::Column::Provider.eq(provider))
            .filter(oauth_accounts::Column::UserId.eq(*user_id))
            .one(&self.database)
            .await?
            .map(Into::into))
    }

    async fn list_oauth_accounts(&self, user_id: &Uuid) -> anyhow::Result<Vec<OAuthAccount>> {
        Ok(oauth_accounts::Entity::find()
            .filter(oauth_accounts::Column::UserId.eq(*user_id))
            .all(&self.database)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn insert_oauth_account(
        &self,
        user_id: &Uuid,
        provider: String,
        provider_subject: String,
        email: String,
        display_name: Option<String>,
        now: DateTime<Utc>,
    ) -> anyhow::Result<OAuthAccount> {
        Ok(oauth_accounts::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(*user_id),
            provider: Set(provider),
            provider_subject: Set(provider_subject),
            email: Set(email),
            display_name: Set(display_name),
            linked_at: Set(now),
        }
        .insert(&self.database)
        .await?
        .into())
    }

    async fn delete_oauth_account(&self, provider: &str, user_id: &Uuid) -> anyhow::Result<bool> {
        let result = oauth_accounts::Entity::delete_many()
            .filter(oauth_accounts::Column::Provider.eq(provider))
            .filter(oauth_accounts::Column::UserId.eq(*user_id))
            .exec(&self.database)
            .await?;

        Ok(result.rows_affected > 0)
    }

    async fn insert_oauth_handoff(
        &self,
        code_hash: String,
        kind: String,
        user_id: Option<Uuid>,
        registration_intent_id: Option<Uuid>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        oauth_handoffs::ActiveModel {
            id: Set(Uuid::new_v4()),
            code_hash: Set(code_hash),
            kind: Set(kind),
            user_id: Set(user_id),
            registration_intent_id: Set(registration_intent_id),
            created_at: Set(now),
            expires_at: Set(expires_at),
            consumed_at: Set(None),
        }
        .insert(&self.database)
        .await?;

        Ok(())
    }

    async fn find_active_oauth_handoff(
        &self,
        code_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthHandoff>> {
        Ok(oauth_handoffs::Entity::find()
            .filter(oauth_handoffs::Column::CodeHash.eq(code_hash))
            .filter(oauth_handoffs::Column::ConsumedAt.is_null())
            .filter(oauth_handoffs::Column::ExpiresAt.gt(now))
            .one(&self.database)
            .await?
            .map(Into::into))
    }

    async fn consume_oauth_handoff(
        &self,
        handoff_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        if let Some(handoff) = oauth_handoffs::Entity::find_by_id(*handoff_id)
            .one(&self.database)
            .await?
        {
            let mut handoff = handoff.into_active_model();
            handoff.consumed_at = Set(Some(now));
            handoff.update(&self.database).await?;
        }

        Ok(())
    }

    async fn insert_oauth_registration_intent(
        &self,
        provider: String,
        provider_subject: String,
        email: String,
        display_name: Option<String>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<OAuthRegistrationIntent> {
        Ok(oauth_registration_intents::ActiveModel {
            id: Set(Uuid::new_v4()),
            provider: Set(provider),
            provider_subject: Set(provider_subject),
            email: Set(email),
            display_name: Set(display_name),
            created_at: Set(now),
            expires_at: Set(expires_at),
            consumed_at: Set(None),
        }
        .insert(&self.database)
        .await?
        .into())
    }

    async fn find_active_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthRegistrationIntent>> {
        Ok(oauth_registration_intents::Entity::find_by_id(*intent_id)
            .filter(oauth_registration_intents::Column::ConsumedAt.is_null())
            .filter(oauth_registration_intents::Column::ExpiresAt.gt(now))
            .one(&self.database)
            .await?
            .map(Into::into))
    }

    async fn consume_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        if let Some(intent) = oauth_registration_intents::Entity::find_by_id(*intent_id)
            .one(&self.database)
            .await?
        {
            let mut intent = intent.into_active_model();
            intent.consumed_at = Set(Some(now));
            intent.update(&self.database).await?;
        }

        Ok(())
    }
}

fn map_insert_user_error(error: sea_orm::DbErr) -> InsertUserError {
    let message = error.to_string();
    if message.contains("users_nickname_key") {
        return InsertUserError::Conflict(UserConflict::Nickname);
    }
    if message.contains("users_email_normalized_key") {
        return InsertUserError::Conflict(UserConflict::Email);
    }

    InsertUserError::Database(error)
}

//! Infrastructure model conversions.

use crate::features::auth::domain::{
    OAuthAccount, OAuthHandoff, OAuthRegistrationIntent, PasswordResetToken, UserAccount,
};
use crate::features::auth::infrastructure::entities::{
    oauth_accounts, oauth_handoffs, oauth_registration_intents, password_reset_tokens, users,
};

impl From<users::Model> for UserAccount {
    fn from(row: users::Model) -> Self {
        Self {
            id: row.id,
            nickname: row.nickname,
            email: row.email,
            password_hash: row.password_hash,
            registered_at: row.registered_at,
            nickname_updated_at: row.nickname_updated_at,
        }
    }
}

impl From<oauth_accounts::Model> for OAuthAccount {
    fn from(row: oauth_accounts::Model) -> Self {
        Self {
            user_id: row.user_id,
            provider: row.provider,
            provider_subject: row.provider_subject,
            email: row.email,
            display_name: row.display_name,
            linked_at: row.linked_at,
        }
    }
}

impl From<oauth_handoffs::Model> for OAuthHandoff {
    fn from(row: oauth_handoffs::Model) -> Self {
        Self {
            id: row.id,
            kind: row.kind,
            user_id: row.user_id,
            registration_intent_id: row.registration_intent_id,
        }
    }
}

impl From<oauth_registration_intents::Model> for OAuthRegistrationIntent {
    fn from(row: oauth_registration_intents::Model) -> Self {
        Self {
            id: row.id,
            provider_subject: row.provider_subject,
            email: row.email,
            display_name: row.display_name,
        }
    }
}

impl From<password_reset_tokens::Model> for PasswordResetToken {
    fn from(row: password_reset_tokens::Model) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
        }
    }
}

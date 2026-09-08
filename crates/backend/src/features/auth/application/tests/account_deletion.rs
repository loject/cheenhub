//! Проверки удаления аккаунта, срока восстановления и отзыва доступа.

use cheenhub_contracts::rest::{AccountRestoreRequest, LoginRequest, RefreshRequest};
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::{registered_user, state_with_mailer};
use crate::features::auth::{
    application::{delete_current_user, login, me, refresh_with_user_agent, restore_account},
    email::tests::TestAuthMailer,
    error::AuthError,
    security::refresh_token,
};

fn restore_token(mailer: &TestAuthMailer) -> String {
    let sent = mailer.account_deletion();
    url::Url::parse(
        &sent
            .last()
            .expect("уведомление должно быть отправлено")
            .restore_url,
    )
    .expect("ссылка должна быть валидной")
    .query_pairs()
    .find(|(key, _)| key == "token")
    .expect("ссылка должна содержать токен")
    .1
    .into_owned()
}

fn credentials(email: &str) -> LoginRequest {
    LoginRequest {
        email: email.to_owned(),
        password: "password123".to_owned(),
    }
}

#[tokio::test]
async fn deletion_emails_deadline_revokes_access_and_restores_only_explicitly() {
    let (state, mailer) = state_with_mailer();
    let auth = registered_user(&state, "delete_restore", "restore@example.com").await;
    let user_id = Uuid::parse_str(&auth.user.id).unwrap();
    let disconnected = super::realtime::register_test_session(&state, &auth).await;
    let started = Utc::now();
    let response = delete_current_user(&state, &auth.access_token)
        .await
        .unwrap();
    let deadline = DateTime::parse_from_rfc3339(&response.restore_until).unwrap();
    assert!(deadline >= started + Duration::days(30));
    assert!(deadline <= Utc::now() + Duration::days(30));
    let sent = mailer.account_deletion();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "restore@example.com");
    assert_eq!(
        sent[0].restore_until,
        deadline.format("%d.%m.%Y %H:%M:%S UTC").to_string()
    );
    assert!(
        state
            .auth_store
            .account_deletion(&user_id)
            .await
            .unwrap()
            .is_some()
    );
    assert!(*disconnected.borrow());
    assert!(me(&state, &auth.access_token).await.is_err());
    assert!(
        login(&state, credentials("restore@example.com"))
            .await
            .is_err()
    );
    assert!(
        refresh_with_user_agent(
            &state,
            RefreshRequest {
                refresh_token: auth.refresh_token.clone()
            },
            None
        )
        .await
        .is_err()
    );

    let token = restore_token(&mailer);
    restore_account(
        &state,
        AccountRestoreRequest {
            token: token.clone(),
        },
    )
    .await
    .unwrap();
    assert!(
        state
            .auth_store
            .account_deletion(&user_id)
            .await
            .unwrap()
            .is_none()
    );
    let restored = login(&state, credentials("restore@example.com"))
        .await
        .unwrap();
    assert_eq!(restored.user.id, auth.user.id);
    assert!(me(&state, &auth.access_token).await.is_err());
    assert!(
        refresh_with_user_agent(
            &state,
            RefreshRequest {
                refresh_token: auth.refresh_token
            },
            None
        )
        .await
        .is_err()
    );
    assert!(
        restore_account(&state, AccountRestoreRequest { token })
            .await
            .is_err()
    );
}

#[tokio::test]
async fn restoration_rejects_expired_token_and_exact_deadline() {
    let (state, _) = state_with_mailer();
    let auth = registered_user(&state, "expired_delete", "expired-delete@example.com").await;
    let user_id = Uuid::parse_str(&auth.user.id).unwrap();
    let token = refresh_token::generate();
    let token_hash = refresh_token::hash(&token);
    let deadline = Utc::now() - Duration::seconds(1);
    assert!(
        state
            .auth_store
            .begin_account_deletion(
                &user_id,
                token_hash.clone(),
                deadline - Duration::days(30),
                deadline
            )
            .await
            .unwrap()
    );
    assert!(
        !state
            .auth_store
            .restore_account(&token_hash, deadline)
            .await
            .unwrap()
    );
    assert!(
        restore_account(&state, AccountRestoreRequest { token })
            .await
            .is_err()
    );
    assert!(
        login(&state, credentials("expired-delete@example.com"))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn server_owner_cannot_delete_account_or_receive_restore_email() {
    let (state, mailer) = state_with_mailer();
    let auth = registered_user(&state, "delete_owner", "delete-owner@example.com").await;
    let user_id = Uuid::parse_str(&auth.user.id).unwrap();
    state
        .server_store
        .insert_server(&user_id, "Мой сервер".to_owned())
        .await
        .unwrap();
    let error = delete_current_user(&state, &auth.access_token)
        .await
        .unwrap_err();
    assert!(matches!(error, AuthError::Conflict(_)));
    assert!(mailer.account_deletion().is_empty());
    assert!(
        state
            .auth_store
            .account_deletion(&user_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(me(&state, &auth.access_token).await.is_ok());
}

#[tokio::test]
async fn email_delivery_failure_keeps_account_and_session_active() {
    let (state, mailer) = state_with_mailer();
    let auth = registered_user(&state, "delete_mail_fail", "delete-mail-fail@example.com").await;
    let user_id = Uuid::parse_str(&auth.user.id).unwrap();
    mailer.fail_account_deletion();
    assert!(
        delete_current_user(&state, &auth.access_token)
            .await
            .is_err()
    );
    assert!(mailer.account_deletion().is_empty());
    assert!(
        state
            .auth_store
            .account_deletion(&user_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(me(&state, &auth.access_token).await.is_ok());
    assert!(
        login(&state, credentials("delete-mail-fail@example.com"))
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn passwordless_account_can_restore_without_creating_a_password() {
    let (state, mailer) = state_with_mailer();
    let auth = super::google_only_user(&state).await;
    let user_id = Uuid::parse_str(&auth.user.id).unwrap();
    delete_current_user(&state, &auth.access_token)
        .await
        .unwrap();
    assert!(me(&state, &auth.access_token).await.is_err());
    restore_account(
        &state,
        AccountRestoreRequest {
            token: restore_token(&mailer),
        },
    )
    .await
    .unwrap();
    let user = state
        .auth_store
        .find_user_by_id(&user_id)
        .await
        .unwrap()
        .unwrap();
    assert!(user.password_hash.is_none());
    assert!(
        state
            .auth_store
            .find_oauth_account_for_user("google", &user_id)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        state
            .auth_store
            .create_session(
                &user_id,
                "new-google-session".to_owned(),
                None,
                Utc::now(),
                Utc::now() + Duration::days(1)
            )
            .await
            .is_ok()
    );
    assert!(me(&state, &auth.access_token).await.is_err());
}

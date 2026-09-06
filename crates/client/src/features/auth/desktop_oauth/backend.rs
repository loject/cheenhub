//! Контракт desktop-реализации OAuth.

use super::Progress;
use crate::features::auth::api::OAuthCompletion;
use cheenhub_contracts::rest::OAuthFlow;

/// Платформенный способ запуска и ожидания входа.
pub(super) trait DesktopOAuthBackend {
    /// Сообщает, поддерживается ли desktop-поток.
    fn is_supported() -> bool;
    /// Выполняет вход до завершения либо отмены вызывающей задачи.
    async fn authenticate(
        flow: OAuthFlow,
        on_progress: impl FnMut(Progress),
    ) -> Result<OAuthCompletion, String>;
}

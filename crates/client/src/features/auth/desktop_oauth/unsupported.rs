//! Заглушка desktop OAuth для браузера и мобильного приложения.

use super::super::Progress;
use super::super::backend::DesktopOAuthBackend;
use crate::features::auth::api::OAuthCompletion;
use cheenhub_contracts::rest::OAuthFlow;

/// Реализация неподдерживаемой платформы.
pub(in crate::features::auth::desktop_oauth) struct Platform;
impl DesktopOAuthBackend for Platform {
    fn is_supported() -> bool {
        false
    }
    async fn authenticate(
        _flow: OAuthFlow,
        _on_progress: impl FnMut(Progress),
    ) -> Result<OAuthCompletion, String> {
        Err("Открой установленное приложение CheenHub и повтори вход.".to_owned())
    }
}

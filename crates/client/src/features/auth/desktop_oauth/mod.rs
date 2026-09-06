//! Вход desktop-клиента через браузер с получением результата по REST.

use super::api::OAuthCompletion;
use cheenhub_contracts::rest::OAuthFlow;

/// Стадия попытки, определяющая доступность отмены в интерфейсе.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Progress {
    /// Создание попытки и открытие браузера.
    Opening,
    /// Ожидание подтверждения Google.
    Waiting,
    /// Одноразовое завершение входа на сервере.
    Completing,
}

mod backend;
mod native;
use backend::DesktopOAuthBackend;

/// Проверяет доступность desktop-передачи результата OAuth.
pub(crate) fn is_supported() -> bool {
    native::Platform::is_supported()
}

/// Открывает Google и ожидает результат в рамках задачи вызывающего компонента.
pub(crate) async fn authenticate(
    flow: OAuthFlow,
    on_progress: impl FnMut(Progress),
) -> Result<OAuthCompletion, String> {
    native::Platform::authenticate(flow, on_progress).await
}

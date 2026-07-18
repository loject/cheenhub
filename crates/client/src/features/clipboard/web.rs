//! Web/mobile-реализация текстового буфера обмена через document eval.

use dioxus::prelude::*;

/// Копирует текст через Clipboard API текущего webview или браузера.
pub(super) async fn copy_text(text: String) -> Result<(), String> {
    let eval = document::eval(
        r#"
        const text = await dioxus.recv();
        await navigator.clipboard.writeText(text);
        return true;
        "#,
    );
    eval.send(text)
        .map_err(|_| "Не удалось подготовить копирование.".to_owned())?;
    eval.join::<bool>()
        .await
        .map(|_| ())
        .map_err(|_| "Приложение не разрешило скопировать ссылку.".to_owned())
}

//! Web-реализация PWA-интеграции.
#![cfg_attr(not(target_family = "wasm"), allow(dead_code, unused_imports))]

use dioxus::prelude::*;

/// Передает build-версию приложения в статичный PWA-регистратор.
#[component]
pub(crate) fn PwaVersionBridge() -> Element {
    let app_version = env!("CHEENHUB_APP_VERSION");

    use_effect(move || {
        let script = format!(
            r#"
            window.dispatchEvent(new CustomEvent("cheenhub:pwa-version", {{
                detail: {{ version: "{}" }}
            }}));
            "#,
            js_string_literal(app_version)
        );
        document::eval(&script);
        info!(
            app_version,
            "sent application version to pwa register script"
        );
    });

    rsx! {}
}

fn js_string_literal(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| character.escape_default())
        .collect()
}

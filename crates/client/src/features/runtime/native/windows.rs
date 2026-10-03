/// Возвращает аргументы WebView2 для secure context внутренней страницы клиента.
pub(super) fn webview_browser_arguments() -> &'static str {
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --unsafely-treat-insecure-origin-as-secure=http://dioxus.index.html"
}

#[cfg(test)]
mod tests;

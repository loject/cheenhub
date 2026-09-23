/// Возвращает аргументы WebView2 для secure context внутренней страницы клиента.
pub(super) fn webview_browser_arguments() -> &'static str {
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --unsafely-treat-insecure-origin-as-secure=http://dioxus.index.html"
}

#[cfg(test)]
mod tests {
    use super::webview_browser_arguments;

    #[test]
    fn trusts_only_the_internal_dioxus_origin_and_preserves_wry_defaults() {
        let arguments = webview_browser_arguments();

        assert!(
            arguments
                .contains("--unsafely-treat-insecure-origin-as-secure=http://dioxus.index.html")
        );
        assert!(
            arguments.contains("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection")
        );
        assert_eq!(
            arguments
                .matches("--unsafely-treat-insecure-origin-as-secure=")
                .count(),
            1
        );
    }
}

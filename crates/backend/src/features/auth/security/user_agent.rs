//! User-Agent normalization helpers.

const MAX_USER_AGENT_CHARS: usize = 512;

/// Coarse device type inferred from a User-Agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParsedDeviceKind {
    /// Desktop or laptop browser.
    Desktop,
    /// Phone browser or mobile app web view.
    Mobile,
    /// Tablet browser or tablet app web view.
    Tablet,
    /// Automated client, crawler, or script-like runtime.
    Bot,
    /// Unknown client type.
    Unknown,
}

/// User-facing User-Agent details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedUserAgent {
    /// Inferred device category.
    pub(crate) device_kind: ParsedDeviceKind,
    /// Human-readable operating system name.
    pub(crate) os_name: String,
    /// Human-readable browser or client name.
    pub(crate) browser_name: String,
}

/// Returns a bounded User-Agent value suitable for persistence.
pub(crate) fn normalize(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.chars().take(MAX_USER_AGENT_CHARS).collect())
}

/// Parses a normalized User-Agent into stable user-facing labels.
pub(crate) fn parse(value: Option<&str>) -> ParsedUserAgent {
    let Some(value) = value else {
        return unknown_user_agent();
    };
    let Some(normalized) = normalize(value) else {
        return unknown_user_agent();
    };
    let lower = normalized.to_ascii_lowercase();

    ParsedUserAgent {
        device_kind: parse_device_kind(&lower),
        os_name: parse_os_name(&lower).to_owned(),
        browser_name: parse_browser_name(&lower).to_owned(),
    }
}

fn unknown_user_agent() -> ParsedUserAgent {
    ParsedUserAgent {
        device_kind: ParsedDeviceKind::Unknown,
        os_name: "Неизвестная ОС".to_owned(),
        browser_name: "Неизвестный браузер".to_owned(),
    }
}

fn parse_device_kind(lower: &str) -> ParsedDeviceKind {
    if is_bot(lower) {
        return ParsedDeviceKind::Bot;
    }
    if lower.contains("ipad") || lower.contains("tablet") {
        return ParsedDeviceKind::Tablet;
    }
    if lower.contains("iphone")
        || lower.contains("ipod")
        || lower.contains("mobile")
        || (lower.contains("android") && !lower.contains("tablet"))
    {
        return ParsedDeviceKind::Mobile;
    }
    if lower.contains("windows")
        || lower.contains("macintosh")
        || lower.contains("x11")
        || lower.contains("linux")
        || lower.contains("cros")
    {
        return ParsedDeviceKind::Desktop;
    }

    ParsedDeviceKind::Unknown
}

fn parse_os_name(lower: &str) -> &'static str {
    if lower.contains("windows nt") || lower.contains("windows") {
        return "Windows";
    }
    if lower.contains("iphone") || lower.contains("ipad") || lower.contains("ipod") {
        return "iOS";
    }
    if lower.contains("android") {
        return "Android";
    }
    if lower.contains("mac os x") || lower.contains("macintosh") {
        return "macOS";
    }
    if lower.contains("cros") {
        return "ChromeOS";
    }
    if lower.contains("ubuntu") {
        return "Ubuntu";
    }
    if lower.contains("linux") || lower.contains("x11") {
        return "Linux";
    }

    "Неизвестная ОС"
}

fn parse_browser_name(lower: &str) -> &'static str {
    if is_bot(lower) {
        return "Бот";
    }
    if lower.contains("edg/") || lower.contains("edge/") {
        return "Microsoft Edge";
    }
    if lower.contains("opr/") || lower.contains("opera/") {
        return "Opera";
    }
    if lower.contains("samsungbrowser/") {
        return "Samsung Internet";
    }
    if lower.contains("firefox/") || lower.contains("fxios/") {
        return "Firefox";
    }
    if lower.contains("crios/") || lower.contains("chrome/") || lower.contains("chromium/") {
        return "Chrome";
    }
    if lower.contains("safari/") {
        return "Safari";
    }

    "Неизвестный браузер"
}

fn is_bot(lower: &str) -> bool {
    lower.contains("bot")
        || lower.contains("crawler")
        || lower.contains("spider")
        || lower.contains("slurp")
        || lower.contains("curl/")
        || lower.contains("wget/")
}

#[cfg(test)]
mod tests {
    use super::{ParsedDeviceKind, parse};

    #[test]
    fn parses_desktop_chrome_on_linux() {
        let parsed = parse(Some(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        ));

        assert_eq!(parsed.device_kind, ParsedDeviceKind::Desktop);
        assert_eq!(parsed.os_name, "Linux");
        assert_eq!(parsed.browser_name, "Chrome");
    }

    #[test]
    fn parses_mobile_safari_on_ios() {
        let parsed = parse(Some(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) \
             AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 \
             Mobile/15E148 Safari/604.1",
        ));

        assert_eq!(parsed.device_kind, ParsedDeviceKind::Mobile);
        assert_eq!(parsed.os_name, "iOS");
        assert_eq!(parsed.browser_name, "Safari");
    }

    #[test]
    fn parses_automated_clients_as_bots() {
        let parsed = parse(Some("curl/8.4.0"));

        assert_eq!(parsed.device_kind, ParsedDeviceKind::Bot);
        assert_eq!(parsed.browser_name, "Бот");
    }
}

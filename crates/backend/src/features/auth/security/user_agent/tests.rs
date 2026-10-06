//! Тесты разбора User-Agent.

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
fn parses_native_cheenhub_on_windows() {
    let parsed = parse(Some("CheenHub/0.1.0 (Windows)"));

    assert_eq!(parsed.device_kind, ParsedDeviceKind::Desktop);
    assert_eq!(parsed.os_name, "Windows");
    assert_eq!(parsed.browser_name, "CheenHub");
}

#[test]
fn parses_native_cheenhub_on_linux() {
    let parsed = parse(Some("CheenHub/0.1.0 (Linux)"));

    assert_eq!(parsed.device_kind, ParsedDeviceKind::Desktop);
    assert_eq!(parsed.os_name, "Linux");
    assert_eq!(parsed.browser_name, "CheenHub");
}

#[test]
fn parses_native_cheenhub_on_macos() {
    let parsed = parse(Some("CheenHub/0.1.0 (macOS)"));

    assert_eq!(parsed.device_kind, ParsedDeviceKind::Desktop);
    assert_eq!(parsed.os_name, "macOS");
    assert_eq!(parsed.browser_name, "CheenHub");
}

#[test]
fn parses_native_cheenhub_on_android() {
    let parsed = parse(Some("CheenHub/0.1.0 (Android)"));

    assert_eq!(parsed.device_kind, ParsedDeviceKind::Mobile);
    assert_eq!(parsed.os_name, "Android");
    assert_eq!(parsed.browser_name, "CheenHub");
}

#[test]
fn parses_automated_clients_as_bots() {
    let parsed = parse(Some("curl/8.4.0"));

    assert_eq!(parsed.device_kind, ParsedDeviceKind::Bot);
    assert_eq!(parsed.browser_name, "Бот");
}

//! Единая проверка адреса перед отображением и открытием.

pub(super) fn checked(value: &str) -> Option<url::Url> {
    if value.chars().any(|ch| ch.is_whitespace() || ch.is_control() || ch == '\\' || matches!(ch, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')) {
        return None;
    }
    let (scheme, authority) = value.split_once("://")?;
    if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https")
        || authority.is_empty()
        || authority.starts_with(['/', '?', '#'])
        || authority.split(['/', '?', '#']).next()?.contains('@')
    {
        return None;
    }
    let bytes = value.as_bytes();
    let lower = value.to_ascii_lowercase();
    if [
        "%e2%80%8e",
        "%e2%80%8f",
        "%e2%80%aa",
        "%e2%80%ab",
        "%e2%80%ac",
        "%e2%80%ad",
        "%e2%80%ae",
        "%e2%81%a6",
        "%e2%81%a7",
        "%e2%81%a8",
        "%e2%81%a9",
    ]
    .iter()
    .any(|control| lower.contains(control))
    {
        return None;
    }
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'%' {
            let hex = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
            let decoded = u8::from_str_radix(hex, 16).ok()?;
            if decoded < 0x20 || decoded == 0x7f || decoded == b'\\' {
                return None;
            }
        }
    }
    let url = url::Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    Some(url)
}

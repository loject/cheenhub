//! Ключи native-устройств ввода для `cpal`.

const INPUT_DEVICE_PREFIX: &str = "cpal-input";

/// Разбирает ключ устройства ввода, созданный `input_device_id`.
pub(in crate::features::microphone) fn parse_input_device_id(
    device_id: &str,
) -> Option<(usize, &str)> {
    let rest = device_id
        .strip_prefix(INPUT_DEVICE_PREFIX)?
        .strip_prefix(':')?;
    let (ordinal, label) = rest.split_once(':')?;
    let ordinal = ordinal.parse::<usize>().ok()?;
    if label.is_empty() {
        return None;
    }

    Some((ordinal, label))
}

#[cfg(test)]
mod tests {
    use super::parse_input_device_id;

    #[test]
    fn parses_native_input_device_key() {
        let device_id = "cpal-input:7:Line In: USB";

        assert_eq!(parse_input_device_id(&device_id), Some((7, "Line In: USB")));
    }

    #[test]
    fn rejects_legacy_or_malformed_device_key() {
        assert_eq!(parse_input_device_id("Microphone"), None);
        assert_eq!(parse_input_device_id("cpal-input:name"), None);
        assert_eq!(parse_input_device_id("cpal-input:2:"), None);
    }
}

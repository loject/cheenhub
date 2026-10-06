use super::parse_realtime_cert_sha256;

#[test]
fn принимает_fingerprint_из_32_байт_с_разделителями() {
    let fingerprint = "AB:CD:EF:01:23:45:67:89:AB:CD:EF:01:23:45:67:89:AB:CD:EF:01:23:45:67:89:AB:CD:EF:01:23:45:67:89";

    assert_eq!(
        parse_realtime_cert_sha256(fingerprint).unwrap(),
        Some(vec![
            0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45,
            0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01,
            0x23, 0x45, 0x67, 0x89,
        ])
    );
}

#[test]
fn отклоняет_fingerprint_неверной_длины() {
    let error = parse_realtime_cert_sha256("ab").unwrap_err();

    assert!(error.to_string().contains("ровно 64"));
}

#[test]
fn отклоняет_fingerprint_не_в_hex_формате() {
    let error = parse_realtime_cert_sha256(&"z".repeat(64)).unwrap_err();

    assert!(error.to_string().contains("корректным hex"));
}

#[test]
fn принимает_только_пустой_fingerprint_как_не_настроенный() {
    assert_eq!(parse_realtime_cert_sha256("").unwrap(), None);
    assert!(parse_realtime_cert_sha256(" \n:").is_err());
}

use super::decode_chunked;

#[test]
fn decodes_chunked_http_body() {
    let decoded = decode_chunked(b"4\r\ntest\r\n3\r\n123\r\n0\r\n\r\n").expect("body decodes");
    assert_eq!(decoded, b"test123");
}

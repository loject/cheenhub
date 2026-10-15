//! Unit-тесты разбора служебных ответов Docker и HTTP.

use super::{parse_headers, parse_port};

#[test]
fn parses_headers_from_last_http_response() {
    let headers = parse_headers(
        "HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 413 Request Entity Too Large\r\nCache-Control: no-cache\r\n\r\n",
    );

    assert_eq!(headers, [("Cache-Control".into(), "no-cache".into())]);
}

#[test]
fn parses_loopback_docker_port() {
    assert_eq!(parse_port("127.0.0.1:49152\n").unwrap(), 49152);
}

#[test]
fn rejects_invalid_docker_port() {
    assert!(parse_port("unavailable").is_err());
    assert!(parse_port("127.0.0.1:not-a-port").is_err());
}

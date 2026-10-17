use super::{
    api_base_url, parse_edge_http_port, parse_https_port, parse_webtransport_port, socket_addr_from,
};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

#[test]
fn wraps_ipv6_host_into_brackets() {
    let address = socket_addr_from("::", 3000).expect("ipv6 any address");
    assert_eq!(address, SocketAddr::from((Ipv6Addr::UNSPECIFIED, 3000)));
}

#[test]
fn keeps_ipv4_host_without_brackets() {
    let address = socket_addr_from("0.0.0.0", 3000).expect("ipv4 any address");
    assert_eq!(address, SocketAddr::from((Ipv4Addr::UNSPECIFIED, 3000)));
}

#[test]
fn accepts_already_bracketed_ipv6_host() {
    let address = socket_addr_from("[::1]", 3000).expect("bracketed ipv6");
    assert_eq!(address, SocketAddr::from((Ipv6Addr::LOCALHOST, 3000)));
}

#[test]
fn rejects_invalid_host() {
    assert!(socket_addr_from("not-a-host", 3000).is_err());
}

#[test]
fn derives_api_url_from_service_base_url() {
    assert_eq!(
        api_base_url("http://192.168.2.2:3000").expect("публичный API URL должен собираться"),
        "http://192.168.2.2:3000/api"
    );
    assert_eq!(
        api_base_url("https://cheenhub.test/").expect("публичный API URL должен собираться"),
        "https://cheenhub.test/api"
    );
}

#[test]
fn rejects_base_url_with_path_credentials_or_unsupported_scheme() {
    assert!(api_base_url("https://cheenhub.test/root").is_err());
    assert!(api_base_url("https://user:secret@cheenhub.test").is_err());
    assert!(api_base_url("ftp://cheenhub.test").is_err());
}

#[test]
fn https_port_is_disabled_by_default_and_accepts_a_distinct_port() {
    assert_eq!(parse_https_port(None, 3000).unwrap(), None);
    assert_eq!(parse_https_port(Some("3443"), 3000).unwrap(), Some(3443));
}

#[test]
fn https_port_rejects_invalid_or_conflicting_values() {
    for value in ["0", "3000", "65536", "not-a-port"] {
        assert!(parse_https_port(Some(value), 3000).is_err(), "{value}");
    }
}

#[test]
fn webtransport_port_defaults_to_http_port_and_accepts_independent_udp_port() {
    assert_eq!(parse_webtransport_port(None, 3000).unwrap(), 3000);
    assert_eq!(parse_webtransport_port(Some("443"), 3000).unwrap(), 443);
    assert_eq!(parse_webtransport_port(Some("443"), 443).unwrap(), 443);
    assert_eq!(
        socket_addr_from("::", parse_webtransport_port(Some("443"), 3000).unwrap()).unwrap(),
        SocketAddr::from((Ipv6Addr::UNSPECIFIED, 443))
    );
}

#[test]
fn webtransport_port_rejects_invalid_or_zero_values() {
    for value in ["0", "65536", "invalid", "-1"] {
        assert!(
            parse_webtransport_port(Some(value), 3000).is_err(),
            "{value}"
        );
    }
}

#[test]
fn edge_port_requires_https_and_distinct_port() {
    assert_eq!(parse_edge_http_port(None, 3000, None).unwrap(), None);
    assert_eq!(
        parse_edge_http_port(Some("8080"), 3000, Some(3443)).unwrap(),
        Some(8080)
    );
    for (port, https) in [
        ("8080", None),
        ("0", Some(3443)),
        ("3000", Some(3443)),
        ("3443", Some(3443)),
        ("invalid", Some(3443)),
    ] {
        assert!(
            parse_edge_http_port(Some(port), 3000, https).is_err(),
            "{port}"
        );
    }
}

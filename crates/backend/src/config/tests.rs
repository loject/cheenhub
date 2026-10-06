use super::{api_base_url, socket_addr_from};
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

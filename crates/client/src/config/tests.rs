use super::{
    api_url_from_base, realtime_websocket_url_from_base, realtime_webtransport_url_from_base,
};

#[test]
fn derives_all_endpoints_from_http_base_url() {
    let base_url = "http://192.168.2.2:3000";

    assert_eq!(
        api_url_from_base(base_url, "/auth/sessions")
            .expect("REST URL должен собираться")
            .as_str(),
        "http://192.168.2.2:3000/api/auth/sessions"
    );
    assert_eq!(
        api_url_from_base(base_url, "/friends/search?q=alice%20smith")
            .expect("REST URL с query string должен собираться")
            .as_str(),
        "http://192.168.2.2:3000/api/friends/search?q=alice%20smith"
    );
    assert_eq!(
        realtime_websocket_url_from_base(base_url)
            .expect("WebSocket URL должен собираться")
            .as_str(),
        "ws://192.168.2.2:3000/api/realtime/ws"
    );
    assert_eq!(
        realtime_webtransport_url_from_base(base_url)
            .expect("WebTransport URL должен собираться")
            .as_str(),
        "https://192.168.2.2:3000/realtime"
    );
}

#[test]
fn derives_secure_endpoints_from_https_base_url() {
    let base_url = "https://cheenhub.test:8443/";

    assert_eq!(
        realtime_websocket_url_from_base(base_url)
            .expect("WebSocket URL должен собираться")
            .as_str(),
        "wss://cheenhub.test:8443/api/realtime/ws"
    );
    assert_eq!(
        realtime_webtransport_url_from_base(base_url)
            .expect("WebTransport URL должен собираться")
            .as_str(),
        "https://cheenhub.test:8443/realtime"
    );
}

#[test]
fn rejects_base_url_with_path_or_unsupported_scheme() {
    assert!(api_url_from_base("https://cheenhub.test/root", "/users").is_err());
    assert!(realtime_websocket_url_from_base("ftp://cheenhub.test").is_err());
}

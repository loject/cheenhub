use super::*;

#[test]
fn transport_notice_uses_only_a_general_vpn_recommendation() {
    let message = fallback_notice(WebTransportFallbackReason::Timeout);

    assert!(message.contains("Если используется VPN или прокси"));
    assert!(!message.contains("На устройстве активен VPN"));
}

#[test]
fn authentication_notice_does_not_blame_udp_or_vpn() {
    let message = fallback_notice(WebTransportFallbackReason::Authentication);

    assert!(!message.contains("UDP"));
    assert!(!message.contains("VPN"));
    assert!(message.contains("не смогла завершить вход"));
}

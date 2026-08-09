//! Уведомление о подтвержденном переходе realtime на резервный транспорт.

use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::realtime::{RealtimeHandle, WebTransportFallbackReason};
use crate::features::toast::ToastHandle;

/// Следит за эпизодами realtime-деградации и показывает одно локальное предупреждение.
#[component]
pub(crate) fn RealtimeFallbackNotice() -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let toast = use_context::<ToastHandle>();

    use_hook(move || {
        let realtime = realtime.clone();
        spawn(async move {
            let mut fallback_events = realtime.subscribe_fallback_info();
            let mut degradation_notice_shown = false;
            while let Some(fallback) = fallback_events.next().await {
                let Some(fallback) = fallback else {
                    degradation_notice_shown = false;
                    continue;
                };
                if degradation_notice_shown {
                    continue;
                }

                toast.warning(fallback_notice(fallback.reason));
                degradation_notice_shown = true;
                warn!(
                    reason = ?fallback.reason,
                    diagnostic_code = fallback.diagnostic_code(),
                    webtransport_elapsed_ms = fallback.webtransport_elapsed_ms,
                    "showed realtime fallback notice"
                );
            }
            debug!("realtime fallback notice subscription closed");
        });
    });

    rsx! {}
}

fn fallback_notice(reason: WebTransportFallbackReason) -> String {
    let mut message = String::from(
        "Соединение работает в резервном режиме через WebSocket. Сообщения доступны, но голос и трансляции могут быть менее стабильными.",
    );
    match reason {
        WebTransportFallbackReason::Timeout
        | WebTransportFallbackReason::Transport
        | WebTransportFallbackReason::Unknown => message.push_str(
            " Если используется VPN или прокси, разрешите UDP/QUIC либо добавьте cheenhub.ru в исключения.",
        ),
        WebTransportFallbackReason::Dns => {
            message.push_str(" Проверьте DNS и доступность cheenhub.ru в текущей сети.");
        }
        WebTransportFallbackReason::Tls => {
            message.push_str(" Проверьте дату устройства и доверие к сертификатам.");
        }
        WebTransportFallbackReason::Authentication => message.push_str(
            " Основной транспорт подключился, но realtime-сессия не смогла завершить вход.",
        ),
    }
    message
}

#[cfg(test)]
mod tests {
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
}

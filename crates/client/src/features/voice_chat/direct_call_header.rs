//! Строка состояния личного звонка в шапке диалога.

use cheenhub_contracts::realtime::DirectCallState;
use chrono::{DateTime, Utc};
use dioxus::prelude::*;

use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle, RealtimeTransportKind};
use crate::features::runtime::sleep_ms;

use super::{DirectCallHandle, VoiceConnectionHandle, VoiceConnectionState, VoiceRoomTarget};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DirectCallHeaderTone {
    Connected,
    Recovering,
    Fallback,
    Error,
}

/// Показывает состояние и длительность личного звонка в подзаголовке диалога.
#[component]
pub(crate) fn DirectCallHeader(
    conversation_id: String,
    peer_user_id: String,
    target: VoiceRoomTarget,
) -> Element {
    let voice = use_context::<VoiceConnectionHandle>();
    let direct_call = use_context::<DirectCallHandle>();
    let realtime = use_context::<RealtimeHandle>();
    let voice_state = voice.state();
    let media_active = voice_state
        .active_target()
        .is_some_and(|active| active.matches(&target));
    let call_visible = media_active || direct_call.is_visible_for_conversation(&conversation_id);
    let call = direct_call.call_for_conversation(&conversation_id);
    let answered_at = call.as_ref().and_then(|call| {
        call.answered_at
            .clone()
            .or_else(|| (call.state == DirectCallState::Active).then(|| call.started_at.clone()))
    });
    let mut elapsed_tick = use_signal(|| 0_u64);

    use_future(move || async move {
        loop {
            sleep_ms(1_000).await;
            elapsed_tick += 1;
        }
    });
    let _elapsed_tick = elapsed_tick();
    let elapsed = call_duration_label(answered_at.as_deref());

    if !call_visible {
        return rsx! {
            p { class: "truncate text-[12px] text-zinc-500", "Личные сообщения" }
        };
    }

    let peer_present = voice_state
        .participants()
        .iter()
        .any(|participant| participant.user_id == peer_user_id);
    let (status, tone) = if call
        .as_ref()
        .is_some_and(|call| call.state == DirectCallState::Ringing)
    {
        if call
            .as_ref()
            .is_some_and(|call| direct_call.is_outgoing(call))
        {
            ("Звоним".to_owned(), DirectCallHeaderTone::Recovering)
        } else {
            (
                "Входящий звонок".to_owned(),
                DirectCallHeaderTone::Recovering,
            )
        }
    } else {
        match &voice_state {
            VoiceConnectionState::Connected {
                target: connected_target,
                ..
            } if connected_target.matches(&target) && peer_present => {
                match realtime.connection_status() {
                    RealtimeConnectionStatus::Connected(
                        RealtimeTransportKind::WebSocketFallback,
                    ) => (
                        "Связь нестабильна".to_owned(),
                        DirectCallHeaderTone::Fallback,
                    ),
                    _ => ("На связи".to_owned(), DirectCallHeaderTone::Connected),
                }
            }
            VoiceConnectionState::Connected {
                target: connected_target,
                ..
            } if connected_target.matches(&target) => (
                "Ждём собеседника".to_owned(),
                DirectCallHeaderTone::Recovering,
            ),
            VoiceConnectionState::Connecting {
                target: connecting_target,
            } if connecting_target.matches(&target) => (
                "Соединяем звонок".to_owned(),
                DirectCallHeaderTone::Recovering,
            ),
            VoiceConnectionState::Error {
                target: error_target,
                ..
            } if error_target
                .as_ref()
                .is_some_and(|error_target| error_target.matches(&target)) =>
            {
                (
                    "Нужна повторная попытка".to_owned(),
                    DirectCallHeaderTone::Error,
                )
            }
            _ => (
                "Восстанавливаем связь".to_owned(),
                DirectCallHeaderTone::Recovering,
            ),
        }
    };
    let indicator_class = match tone {
        DirectCallHeaderTone::Connected => "bg-blue-400 shadow-[0_0_0_3px_rgba(96,165,250,.12)]",
        DirectCallHeaderTone::Recovering => {
            "animate-pulse bg-amber-300 shadow-[0_0_0_3px_rgba(252,211,77,.10)]"
        }
        DirectCallHeaderTone::Fallback => "bg-amber-300 shadow-[0_0_0_3px_rgba(252,211,77,.10)]",
        DirectCallHeaderTone::Error => "bg-red-400 shadow-[0_0_0_3px_rgba(248,113,113,.10)]",
    };

    rsx! {
        div { class: "direct-call-header flex min-w-0 items-center gap-1.5 text-[12px] text-zinc-400",
            span { class: "h-1.5 w-1.5 shrink-0 rounded-full {indicator_class}" }
            span { class: "truncate", "{status}" }
            if let Some(duration) = elapsed {
                span { class: "text-zinc-600", "·" }
                span { class: "shrink-0 tabular-nums text-zinc-300", "{duration}" }
            }
        }
    }
}

fn call_duration_label(answered_at: Option<&str>) -> Option<String> {
    let answered_at = answered_at
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())?
        .with_timezone(&Utc);
    let elapsed_seconds = Utc::now()
        .signed_duration_since(answered_at)
        .num_seconds()
        .max(0) as u64;
    Some(format_duration(elapsed_seconds))
}

fn format_duration(elapsed_seconds: u64) -> String {
    let hours = elapsed_seconds / 3_600;
    let minutes = elapsed_seconds % 3_600 / 60;
    let seconds = elapsed_seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::format_duration;

    #[test]
    fn formats_short_and_long_call_durations() {
        assert_eq!(format_duration(62), "01:02");
        assert_eq!(format_duration(3_661), "1:01:01");
    }
}

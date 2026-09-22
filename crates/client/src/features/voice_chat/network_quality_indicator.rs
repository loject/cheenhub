//! Встроенный индикатор сетевого качества участника голосового общения.

use dioxus::prelude::*;

use crate::features::app::current_user::CurrentUserContext;

use super::network_quality::{
    NetworkQualityFreshness, VoiceNetworkQualityHandle, VoiceNetworkQualityReading,
};
use super::network_quality_publication::{VoiceRttQuality, rtt_quality};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct IndicatorDisclosureState {
    mouse_hovered: bool,
    pinned: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum NetworkQualityTone {
    Good,
    Neutral,
    Degraded,
    Poor,
}

impl IndicatorDisclosureState {
    fn expanded(self) -> bool {
        self.mouse_hovered || self.pinned
    }

    fn set_mouse_hovered(&mut self, hovered: bool) {
        self.mouse_hovered = hovered;
    }

    fn toggle_pinned(&mut self) {
        self.pinned = !self.pinned;
    }

    fn dismiss(&mut self) {
        self.pinned = false;
    }
}

/// Показывает встроенные сетевые метрики текущего пользователя и собеседника.
#[component]
pub(crate) fn VoiceNetworkQualityIndicator(
    participant_user_id: String,
    participant_name: String,
) -> Element {
    let current_user_id = use_context::<CurrentUserContext>().require_user().id;
    let quality = use_context::<VoiceNetworkQualityHandle>();
    let mut disclosure = use_signal(IndicatorDisclosureState::default);
    let expanded = disclosure().expanded();
    let indicator_z = if disclosure().pinned {
        "z-[100]"
    } else {
        "z-30"
    };
    let local = quality.reading_for(&current_user_id);
    let participant = quality.reading_for(&participant_user_id);
    let is_self = participant_user_id == current_user_id;
    let aggregate_freshness = if is_self {
        local.freshness
    } else {
        worst_freshness(local.freshness, participant.freshness)
    };
    let aggregate_tone = if is_self {
        reading_tone(local)
    } else {
        reading_tone(local).max(reading_tone(participant))
    };
    let tone = match aggregate_tone {
        NetworkQualityTone::Good => {
            "bg-zinc-950/88 text-emerald-300 shadow-[0_0_0_1px_rgba(52,211,153,.20),0_10px_28px_rgba(0,0,0,.30)]"
        }
        NetworkQualityTone::Neutral => {
            "bg-zinc-950/88 text-zinc-300 shadow-[0_0_0_1px_rgba(161,161,170,.20),0_10px_28px_rgba(0,0,0,.30)]"
        }
        NetworkQualityTone::Degraded => {
            "bg-zinc-950/92 text-amber-300 shadow-[0_0_0_1px_rgba(251,191,36,.24),0_10px_28px_rgba(0,0,0,.34)]"
        }
        NetworkQualityTone::Poor => {
            "bg-zinc-950/94 text-red-300 shadow-[0_0_0_1px_rgba(248,113,113,.28),0_10px_28px_rgba(0,0,0,.38)]"
        }
    };
    let container_size = if expanded {
        "w-[min(260px,calc(100%_-_1.5rem))] min-h-10 px-3 py-2.5"
    } else {
        "h-10 w-10 px-0 py-0"
    };
    let details_class = if expanded {
        "ml-2.5 min-w-0 flex-1 origin-left scale-100 opacity-100 blur-0 transition-[opacity,transform,filter] duration-200 ease-[cubic-bezier(0.2,0,0,1)]"
    } else {
        "pointer-events-none ml-0 max-w-0 flex-1 origin-left scale-[0.25] overflow-hidden opacity-0 blur-[4px] transition-[opacity,transform,filter] duration-150 ease-[cubic-bezier(0.2,0,0,1)]"
    };
    let aria_label = if expanded {
        "Свернуть сведения о качестве соединения"
    } else {
        "Показать сведения о качестве соединения"
    };

    rsx! {
        if disclosure().pinned {
            div {
                class: "fixed inset-0 z-[99] cursor-default",
                "aria-label": "Закрыть сведения о качестве соединения",
                onclick: move |_| disclosure.write().dismiss(),
            }
        }
        button {
            r#type: "button",
            class: "absolute left-3 top-3 {indicator_z} flex items-center overflow-hidden rounded-xl text-left backdrop-blur-xl transition-[width,min-height,padding,background-color,box-shadow,transform] duration-200 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96] motion-reduce:transition-none {container_size} {tone}",
            "aria-label": aria_label,
            "aria-expanded": if expanded { "true" } else { "false" },
            onpointerenter: move |event| {
                if event.pointer_type() == "mouse" {
                    disclosure.write().set_mouse_hovered(true);
                }
            },
            onpointerleave: move |event| {
                if event.pointer_type() == "mouse" {
                    disclosure.write().set_mouse_hovered(false);
                }
            },
            onclick: move |event| {
                event.stop_propagation();
                disclosure.write().toggle_pinned();
            },
            span { class: "grid h-10 w-10 shrink-0 place-items-center",
                svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 18.5v-3.25" }
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8.5 18.5h7" }
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.25 13.75a4 4 0 0 1 5.5 0" }
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6.5 11a8 8 0 0 1 11 0" }
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3.75 8.25a12 12 0 0 1 16.5 0" }
                }
            }
            span { class: details_class,
                span { class: "flex items-center justify-between gap-3 text-[11px] leading-4",
                    span { class: "truncate font-medium text-zinc-300", "Вы ↔ сервер" }
                    span { class: "shrink-0 font-mono font-semibold tabular-nums text-zinc-100", "{format_rtt(local)}" }
                }
                if !is_self {
                    span { class: "mt-1 flex items-center justify-between gap-3 text-[11px] leading-4",
                        span { class: "truncate font-medium text-zinc-300", "{participant_name} ↔ сервер" }
                        span { class: "shrink-0 font-mono font-semibold tabular-nums text-zinc-100", "{format_rtt(participant)}" }
                    }
                }
                if let Some(label) = status_label(VoiceNetworkQualityReading {
                    rtt_ms: None,
                    freshness: aggregate_freshness,
                }, aggregate_tone) {
                    span { class: "mt-1.5 block text-[10px] font-medium leading-4 text-current", "{label}" }
                }
            }
        }
    }
}

fn worst_freshness(
    first: NetworkQualityFreshness,
    second: NetworkQualityFreshness,
) -> NetworkQualityFreshness {
    use NetworkQualityFreshness::{Fresh, Measuring, Unstable, Waiting};
    match (first, second) {
        (Unstable, _) | (_, Unstable) => Unstable,
        (Waiting, _) | (_, Waiting) => Waiting,
        (Measuring, _) | (_, Measuring) => Measuring,
        (Fresh, Fresh) => Fresh,
    }
}

fn format_rtt(reading: VoiceNetworkQualityReading) -> String {
    reading
        .rtt_ms
        .map(|rtt_ms| format!("{rtt_ms} мс"))
        .unwrap_or_else(|| "—".to_owned())
}

fn reading_tone(reading: VoiceNetworkQualityReading) -> NetworkQualityTone {
    let freshness_tone = match reading.freshness {
        NetworkQualityFreshness::Measuring => NetworkQualityTone::Neutral,
        NetworkQualityFreshness::Fresh => NetworkQualityTone::Good,
        NetworkQualityFreshness::Waiting => NetworkQualityTone::Degraded,
        NetworkQualityFreshness::Unstable => NetworkQualityTone::Poor,
    };
    let rtt_tone = match reading.rtt_ms.map(rtt_quality) {
        Some(VoiceRttQuality::Good) => NetworkQualityTone::Good,
        Some(VoiceRttQuality::Degraded) => NetworkQualityTone::Degraded,
        Some(VoiceRttQuality::Poor) => NetworkQualityTone::Poor,
        None => NetworkQualityTone::Neutral,
    };
    freshness_tone.max(rtt_tone)
}

fn status_label(
    reading: VoiceNetworkQualityReading,
    tone: NetworkQualityTone,
) -> Option<&'static str> {
    match reading.freshness {
        NetworkQualityFreshness::Measuring => Some("Измеряем…"),
        NetworkQualityFreshness::Waiting => Some("Нет свежих данных"),
        NetworkQualityFreshness::Unstable => Some("Соединение нестабильно"),
        NetworkQualityFreshness::Fresh => match tone {
            NetworkQualityTone::Good | NetworkQualityTone::Neutral => None,
            NetworkQualityTone::Degraded => Some("Повышенная задержка"),
            NetworkQualityTone::Poor => Some("Высокая задержка"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        IndicatorDisclosureState, NetworkQualityTone, format_rtt, reading_tone, status_label,
    };
    use crate::features::voice_chat::network_quality::{
        NetworkQualityFreshness, VoiceNetworkQualityReading,
    };

    #[test]
    fn formats_live_and_stale_network_readings() {
        let fresh = VoiceNetworkQualityReading {
            rtt_ms: Some(9),
            freshness: NetworkQualityFreshness::Fresh,
        };
        let waiting = VoiceNetworkQualityReading {
            rtt_ms: Some(42),
            freshness: NetworkQualityFreshness::Waiting,
        };
        let unstable = VoiceNetworkQualityReading {
            rtt_ms: None,
            freshness: NetworkQualityFreshness::Unstable,
        };

        assert_eq!(format_rtt(fresh), "9 мс");
        assert_eq!(format_rtt(waiting), "42 мс");
        assert_eq!(
            status_label(waiting, reading_tone(waiting)),
            Some("Нет свежих данных")
        );
        assert_eq!(
            status_label(unstable, reading_tone(unstable)),
            Some("Соединение нестабильно")
        );
    }

    #[test]
    fn missing_first_rtt_is_neutral_while_measuring() {
        let measuring = VoiceNetworkQualityReading {
            rtt_ms: None,
            freshness: NetworkQualityFreshness::Measuring,
        };

        assert_eq!(reading_tone(measuring), NetworkQualityTone::Neutral);
        assert_eq!(
            status_label(measuring, reading_tone(measuring)),
            Some("Измеряем…")
        );
    }

    #[test]
    fn classifies_fresh_rtt_at_voice_quality_boundaries() {
        let reading = |rtt_ms| VoiceNetworkQualityReading {
            rtt_ms: Some(rtt_ms),
            freshness: NetworkQualityFreshness::Fresh,
        };

        assert_eq!(reading_tone(reading(150)), NetworkQualityTone::Good);
        assert_eq!(reading_tone(reading(151)), NetworkQualityTone::Degraded);
        assert_eq!(reading_tone(reading(500)), NetworkQualityTone::Degraded);
        assert_eq!(reading_tone(reading(501)), NetworkQualityTone::Poor);
        assert_eq!(reading_tone(reading(800)), NetworkQualityTone::Poor);
        assert_eq!(reading_tone(reading(1_400)), NetworkQualityTone::Poor);
    }

    #[test]
    fn freshness_can_only_worsen_the_rtt_tone() {
        let reading = |freshness| VoiceNetworkQualityReading {
            rtt_ms: Some(20),
            freshness,
        };

        assert_eq!(
            reading_tone(reading(NetworkQualityFreshness::Waiting)),
            NetworkQualityTone::Degraded
        );
        assert_eq!(
            reading_tone(reading(NetworkQualityFreshness::Unstable)),
            NetworkQualityTone::Poor
        );
    }

    #[test]
    fn hover_is_temporary_and_click_pins_the_indicator() {
        let mut state = IndicatorDisclosureState::default();
        assert!(!state.expanded());

        state.set_mouse_hovered(true);
        assert!(state.expanded());
        state.set_mouse_hovered(false);
        assert!(!state.expanded());

        state.toggle_pinned();
        assert!(state.expanded());
        state.set_mouse_hovered(true);
        state.set_mouse_hovered(false);
        assert!(state.expanded());
        state.toggle_pinned();
        assert!(!state.expanded());

        state.toggle_pinned();
        assert!(state.expanded());
        state.dismiss();
        assert!(!state.expanded());
    }
}

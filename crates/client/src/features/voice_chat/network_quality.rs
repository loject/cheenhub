//! Состояние качества сети участников голосового общения.

use std::collections::{HashMap, HashSet};

use cheenhub_contracts::realtime::{
    ParticipantNetworkQualityUpdated, PublishVoiceNetworkQuality, RealtimeKind, RealtimeModule,
    VoiceChatKind, VoiceNetworkTargetKind,
};
use dioxus::prelude::*;
use futures_channel::mpsc;
use futures_util::StreamExt;
use web_time::{SystemTime, UNIX_EPOCH};

use crate::features::realtime::{RealtimeError, RealtimeHandle};
use crate::features::{network::NetworkQualityHandle, runtime::sleep_ms};

use super::network_quality_publication::NetworkQualityPublicationState;
use super::realtime::realtime_decode::participant_network_quality;
use super::state::{VoiceConnectionState, VoiceRoomTarget, VoiceRoomTargetKind};

const FRESH_FOR_MS: u64 = 1_500;
const UNSTABLE_AFTER_MS: u64 = 3_000;
const RUNTIME_TICK_MS: u32 = 250;

/// Свежесть последней сетевой метрики участника.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NetworkQualityFreshness {
    /// Первое измерение ещё не получено.
    Measuring,
    /// Метрика получена недавно.
    Fresh,
    /// Новое измерение задерживается.
    Waiting,
    /// Обновления отсутствуют достаточно долго для предупреждения.
    Unstable,
}

/// Отображаемое состояние сетевой метрики одного участника.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VoiceNetworkQualityReading {
    /// Последний известный RTT, если измерение уже получено.
    pub(crate) rtt_ms: Option<u32>,
    /// Свежесть последнего измерения или ожидания первого значения.
    pub(crate) freshness: NetworkQualityFreshness,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VoiceNetworkSample {
    rtt_ms: u32,
    received_at_ms: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct VoiceNetworkQualityState {
    target_key: Option<String>,
    samples: HashMap<String, VoiceNetworkSample>,
}

/// Общий хендл сетевых метрик текущего голосового общения.
#[derive(Clone, Copy)]
pub(crate) struct VoiceNetworkQualityHandle {
    state: Signal<VoiceNetworkQualityState>,
    clock_ms: Signal<u64>,
}

impl VoiceNetworkQualityHandle {
    /// Создаёт хендл на основе сигналов провайдера голосового общения.
    fn new(state: Signal<VoiceNetworkQualityState>, clock_ms: Signal<u64>) -> Self {
        Self { state, clock_ms }
    }

    /// Активирует новую голосовую цель и очищает метрики предыдущей.
    pub(crate) fn activate_target(&mut self, target_key: String, now_ms: u64) {
        self.clock_ms.set(now_ms);
        self.state.with_mut(|state| {
            if state.target_key.as_deref() == Some(target_key.as_str()) {
                return;
            }
            state.target_key = Some(target_key);
            state.samples.clear();
        });
    }

    /// Записывает локально измеренный RTT текущего пользователя.
    pub(crate) fn record_local(&mut self, user_id: String, rtt_ms: u32, now_ms: u64) {
        self.record(user_id, rtt_ms, now_ms);
    }

    /// Применяет полученное от сервера измерение участника.
    pub(crate) fn apply_update(&mut self, update: ParticipantNetworkQualityUpdated, now_ms: u64) {
        self.record(update.user_id, update.rtt_ms, now_ms);
    }

    /// Продвигает общие часы, чтобы UI обновил состояние свежести без нового события.
    pub(crate) fn update_clock(&mut self, now_ms: u64) {
        self.clock_ms.set(now_ms);
    }

    /// Очищает метрики после выхода из голосового общения.
    pub(crate) fn clear(&mut self) {
        self.state.set(VoiceNetworkQualityState::default());
    }

    /// Возвращает состояние сетевой метрики указанного участника.
    pub(crate) fn reading_for(&self, user_id: &str) -> VoiceNetworkQualityReading {
        let state = self.state.read();
        let now_ms = *self.clock_ms.read();
        match state.samples.get(user_id) {
            Some(sample) => VoiceNetworkQualityReading {
                rtt_ms: Some(sample.rtt_ms),
                freshness: freshness_for_age(now_ms.saturating_sub(sample.received_at_ms)),
            },
            None => VoiceNetworkQualityReading {
                rtt_ms: None,
                freshness: NetworkQualityFreshness::Measuring,
            },
        }
    }

    fn record(&mut self, user_id: String, rtt_ms: u32, now_ms: u64) {
        self.clock_ms.set(now_ms);
        self.state.write().samples.insert(
            user_id,
            VoiceNetworkSample {
                rtt_ms,
                received_at_ms: now_ms,
            },
        );
    }

    fn retain_active_participants(&mut self, active_user_ids: &HashSet<String>) {
        self.state.with_mut(|state| {
            retain_active_samples(&mut state.samples, active_user_ids);
        });
    }
}

/// Создаёт состояние сетевых метрик для одного провайдера голосового общения.
pub(super) fn use_voice_network_quality_handle() -> VoiceNetworkQualityHandle {
    let state = use_signal(VoiceNetworkQualityState::default);
    let clock_ms = use_signal(now_ms);
    VoiceNetworkQualityHandle::new(state, clock_ms)
}

/// Запускает получение и публикацию метрик для текущей голосовой цели.
pub(super) fn use_voice_network_quality_runtime(
    state: Signal<VoiceConnectionState>,
    current_user_id: String,
    realtime: RealtimeHandle,
    network_quality: NetworkQualityHandle,
) -> VoiceNetworkQualityHandle {
    let handle = use_voice_network_quality_handle();
    let context_handle = handle;
    use_context_provider(move || context_handle);

    let mut cadence_network_quality = network_quality;
    use_effect(move || {
        cadence_network_quality.set_frequent_probes(connected_target(&state()).is_some());
    });

    let participant_current_user_id = current_user_id.clone();
    let mut participant_quality = handle;
    use_effect(move || {
        let VoiceConnectionState::Connected { participants, .. } = state() else {
            participant_quality.clear();
            return;
        };
        let mut active_user_ids = participants
            .into_iter()
            .map(|participant| participant.user_id)
            .collect::<HashSet<_>>();
        active_user_ids.insert(participant_current_user_id.clone());
        participant_quality.retain_active_participants(&active_user_ids);
    });

    let event_realtime = realtime.clone();
    let mut event_quality = handle;
    use_hook(move || {
        spawn(async move {
            let mut events = subscribe(&event_realtime);
            while let Some(event) = events.next().await {
                let Some(target) = connected_target(&state()) else {
                    continue;
                };
                if !event_matches_target(&event, &target) {
                    continue;
                }
                event_quality.apply_update(event, now_ms());
            }
        })
    });

    let publish_realtime = realtime.clone();
    let mut publish_quality = handle;
    use_hook(move || {
        spawn(async move {
            let mut active_target_key = None::<String>;
            let mut target_activated_at_ms = None::<u64>;
            let mut last_observed_sample_at = None::<u64>;
            let mut publication_state = NetworkQualityPublicationState::default();
            let mut publish_failed = false;
            loop {
                let timestamp_ms = now_ms();
                let Some(target) = connected_target(&state()) else {
                    if active_target_key.take().is_some() {
                        publish_quality.clear();
                        target_activated_at_ms = None;
                        last_observed_sample_at = None;
                        publication_state = NetworkQualityPublicationState::default();
                    }
                    sleep_ms(RUNTIME_TICK_MS).await;
                    continue;
                };
                let next_target_key = target_key(&target);
                if active_target_key.as_deref() != Some(next_target_key.as_str()) {
                    publish_quality.activate_target(next_target_key.clone(), timestamp_ms);
                    active_target_key = Some(next_target_key);
                    target_activated_at_ms = Some(timestamp_ms);
                    last_observed_sample_at = None;
                    publication_state = NetworkQualityPublicationState::default();
                } else {
                    publish_quality.update_clock(timestamp_ms);
                }

                let quality = network_quality.current();
                let Some(sample) = quality.samples.last().copied() else {
                    sleep_ms(RUNTIME_TICK_MS).await;
                    continue;
                };
                if !target_activated_at_ms.is_some_and(|activated_at_ms| {
                    sample_is_newer_than_target(sample.received_at_ms, activated_at_ms)
                }) {
                    sleep_ms(RUNTIME_TICK_MS).await;
                    continue;
                }
                let rtt_ms = sample.rtt_ms.round().clamp(0.0, f64::from(u32::MAX)) as u32;
                if last_observed_sample_at != Some(sample.received_at_ms) {
                    publish_quality.record_local(
                        current_user_id.clone(),
                        rtt_ms,
                        sample.received_at_ms,
                    );
                    last_observed_sample_at = Some(sample.received_at_ms);
                }
                if !publication_state.should_publish(rtt_ms, timestamp_ms) {
                    sleep_ms(RUNTIME_TICK_MS).await;
                    continue;
                }
                match publication_state.finish_attempt(
                    rtt_ms,
                    timestamp_ms,
                    publish(&publish_realtime, rtt_ms).await,
                ) {
                    Ok(()) => {
                        if publish_failed {
                            info!("voice network quality publication recovered");
                            publish_failed = false;
                        }
                    }
                    Err(error) => {
                        if !publish_failed {
                            warn!(%error, "failed to publish voice network quality");
                            publish_failed = true;
                        }
                    }
                }
                sleep_ms(RUNTIME_TICK_MS).await;
            }
        })
    });

    handle
}

/// Возвращает стабильный ключ голосовой цели для сброса метрик при переключении.
pub(super) fn target_key(target: &VoiceRoomTarget) -> String {
    let kind = match target.kind {
        VoiceRoomTargetKind::Server => "server",
        VoiceRoomTargetKind::DirectMessage => "direct_message",
    };
    format!("{kind}:{}:{}", target.server_id, target.room_id)
}

/// Проверяет, относится ли входящая метрика к активной голосовой цели.
pub(super) fn event_matches_target(
    event: &ParticipantNetworkQualityUpdated,
    target: &VoiceRoomTarget,
) -> bool {
    let kind_matches = matches!(
        (event.target_kind, target.kind),
        (VoiceNetworkTargetKind::Server, VoiceRoomTargetKind::Server)
            | (
                VoiceNetworkTargetKind::DirectMessage,
                VoiceRoomTargetKind::DirectMessage
            )
    );
    kind_matches && event.server_id == target.server_id && event.room_id == target.room_id
}

/// Публикует последнее измерение RTT текущего участника до сервера.
pub(super) async fn publish(realtime: &RealtimeHandle, rtt_ms: u32) -> Result<(), RealtimeError> {
    realtime
        .send_reliable(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::PublishNetworkQuality),
            PublishVoiceNetworkQuality { rtt_ms },
        )
        .await
}

/// Подписывает текущую вкладку на сетевые метрики участников голосового общения.
pub(super) fn subscribe(
    realtime: &RealtimeHandle,
) -> mpsc::UnboundedReceiver<ParticipantNetworkQualityUpdated> {
    let events = realtime.subscribe_events();
    let (sender, receiver) = mpsc::unbounded();

    spawn(async move {
        let mut events = events;
        while let Some(envelope) = events.next().await {
            let Some(event) = participant_network_quality(envelope) else {
                continue;
            };
            if sender.unbounded_send(event).is_err() {
                break;
            }
        }
    });

    receiver
}

/// Возвращает текущую клиентскую временную метку в миллисекундах.
pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn freshness_for_age(age_ms: u64) -> NetworkQualityFreshness {
    if age_ms < FRESH_FOR_MS {
        NetworkQualityFreshness::Fresh
    } else if age_ms < UNSTABLE_AFTER_MS {
        NetworkQualityFreshness::Waiting
    } else {
        NetworkQualityFreshness::Unstable
    }
}

fn connected_target(state: &VoiceConnectionState) -> Option<VoiceRoomTarget> {
    match state {
        VoiceConnectionState::Connected { target, .. } => Some(target.clone()),
        VoiceConnectionState::Disconnected
        | VoiceConnectionState::Connecting { .. }
        | VoiceConnectionState::Disconnecting { .. }
        | VoiceConnectionState::Error { .. } => None,
    }
}

fn sample_is_newer_than_target(sample_received_at_ms: u64, target_activated_at_ms: u64) -> bool {
    sample_received_at_ms > target_activated_at_ms
}

fn retain_active_samples(
    samples: &mut HashMap<String, VoiceNetworkSample>,
    active_user_ids: &HashSet<String>,
) {
    samples.retain(|user_id, _| active_user_ids.contains(user_id));
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::{
        NetworkQualityFreshness, VoiceNetworkSample, connected_target, freshness_for_age,
        retain_active_samples, sample_is_newer_than_target,
    };
    use crate::features::voice_chat::state::{VoiceConnectionState, VoiceRoomTarget};

    #[test]
    fn changes_freshness_at_agreed_boundaries() {
        assert_eq!(freshness_for_age(0), NetworkQualityFreshness::Fresh);
        assert_eq!(freshness_for_age(1_499), NetworkQualityFreshness::Fresh);
        assert_eq!(freshness_for_age(1_500), NetworkQualityFreshness::Waiting);
        assert_eq!(freshness_for_age(2_999), NetworkQualityFreshness::Waiting);
        assert_eq!(freshness_for_age(3_000), NetworkQualityFreshness::Unstable);
    }

    #[test]
    fn uses_frequent_probes_only_after_voice_connection_is_established() {
        let target = VoiceRoomTarget::server(
            "server-id".to_string(),
            "room-id".to_string(),
            "Voice room".to_string(),
        );

        assert!(connected_target(&VoiceConnectionState::Disconnected).is_none());
        assert!(
            connected_target(&VoiceConnectionState::Connecting {
                target: target.clone(),
            })
            .is_none()
        );
        assert!(
            connected_target(&VoiceConnectionState::Connected {
                target: target.clone(),
                participants: Vec::new(),
            })
            .is_some()
        );
        assert!(
            connected_target(&VoiceConnectionState::Disconnecting {
                target,
                participants: Vec::new(),
            })
            .is_none()
        );
    }

    #[test]
    fn publishes_only_samples_received_after_target_activation() {
        assert!(!sample_is_newer_than_target(9_999, 10_000));
        assert!(!sample_is_newer_than_target(10_000, 10_000));
        assert!(sample_is_newer_than_target(10_001, 10_000));
    }

    #[test]
    fn removes_samples_for_participants_missing_from_the_latest_snapshot() {
        let mut samples = HashMap::from([
            (
                "current".to_owned(),
                VoiceNetworkSample {
                    rtt_ms: 20,
                    received_at_ms: 1,
                },
            ),
            (
                "active".to_owned(),
                VoiceNetworkSample {
                    rtt_ms: 30,
                    received_at_ms: 2,
                },
            ),
            (
                "left".to_owned(),
                VoiceNetworkSample {
                    rtt_ms: 40,
                    received_at_ms: 3,
                },
            ),
        ]);
        let active_user_ids = HashSet::from(["current".to_owned(), "active".to_owned()]);

        retain_active_samples(&mut samples, &active_user_ids);

        assert_eq!(samples.len(), 2);
        assert!(samples.contains_key("current"));
        assert!(samples.contains_key("active"));
        assert!(!samples.contains_key("left"));
    }
}

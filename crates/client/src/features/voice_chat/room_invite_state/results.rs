//! Срок показа результатов исходящих приглашений и согласование серверного решения.

use std::collections::HashMap;
use std::time::Duration;

use cheenhub_contracts::realtime::{VoiceRoomInviteSnapshot, VoiceRoomInviteStatus};
use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use web_time::Instant;

use super::RoomInviteHandle;
use crate::features::voice_chat::room_invite_tiles::RecentOutgoingResult;

impl RoomInviteHandle {
    /// Показывает новый терминальный статус десять секунд по локальному монотонному таймеру.
    pub(super) fn remember_result(&self, invite: VoiceRoomInviteSnapshot) {
        self.remember_result_for(invite, None);
    }

    /// Восстанавливает только ещё не истёкшую часть показа серверного результата.
    pub(super) fn remember_recovered_result(
        &self,
        invite: VoiceRoomInviteSnapshot,
        server_now: Option<DateTime<Utc>>,
    ) {
        let remaining_ms = remaining_recovered_result_ms(&invite, server_now);
        self.remember_result_for(invite, Some(Duration::from_millis(u64::from(remaining_ms))));
    }

    fn remember_result_for(
        &self,
        invite: VoiceRoomInviteSnapshot,
        recovered_remaining: Option<Duration>,
    ) {
        let invite_id = invite.invite_id.clone();
        let mut current = self.recent_results.peek().clone();
        let previous = current.get(&invite_id).cloned();
        let now = Instant::now();
        let deadline = match recovered_remaining {
            Some(remaining) => {
                update_recent_result_with_duration(&mut current, invite, now, remaining)
            }
            None => update_recent_result(&mut current, invite, now),
        };
        if previous.as_ref() == current.get(&invite_id) {
            return;
        }
        let mut results = self.recent_results;
        results.set(current);
        let Some(deadline) = deadline else {
            debug!(%invite_id, "skipped expired outgoing voice room invitation result");
            return;
        };
        debug!(%invite_id, ?deadline, "updated outgoing voice room invitation result");
        spawn(async move {
            crate::features::runtime::sleep_ms(remaining_ms(deadline, Instant::now())).await;
            let mut current = results.peek().clone();
            if remove_due_result(&mut current, &invite_id, Instant::now()) {
                results.set(current);
            }
        });
    }
}

/// Выбирает завершённые исходящие приглашения из серверного снимка восстановления.
pub(super) fn recovered_outgoing_resolutions(
    recovered: &[VoiceRoomInviteSnapshot],
    current_user_id: &str,
) -> Vec<VoiceRoomInviteSnapshot> {
    recovered
        .iter()
        .filter(|invite| {
            invite.caller_user_id == current_user_id
                && invite.status != VoiceRoomInviteStatus::Pending
        })
        .cloned()
        .collect()
}

/// Не возобновляет уже закончившийся показ предварительного локального тайм-аута.
///
/// Часы устройства и сервера не сравниваются: опираемся на локальное окно показа.
pub(super) fn should_remember_terminal_event(
    previous: Option<&VoiceRoomInviteSnapshot>,
    recent_results: &HashMap<String, RecentOutgoingResult>,
    invite_id: &str,
    now: Instant,
) -> bool {
    let provisional_timeout = previous.is_some_and(|existing| {
        existing.status == VoiceRoomInviteStatus::Expired && existing.resolved_at.is_none()
    });
    !provisional_timeout
        || recent_results
            .get(invite_id)
            .is_some_and(|result| result.show_until > now)
}

/// Остаток десятисекундного показа по двум временным меткам одного сервера.
fn remaining_recovered_result_ms(
    invite: &VoiceRoomInviteSnapshot,
    server_now: Option<DateTime<Utc>>,
) -> u32 {
    let Some(server_now) = server_now else {
        return 10_000;
    };
    let Some(resolved_at) = invite
        .resolved_at
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
    else {
        return 10_000;
    };
    super::expiry_delay_ms(
        resolved_at.with_timezone(&Utc) + chrono::Duration::seconds(10),
        server_now,
    )
    .min(10_000)
}

/// Обновляет кратковременный итог без сравнения часов сервера и устройства.
fn update_recent_result(
    results: &mut HashMap<String, RecentOutgoingResult>,
    invite: VoiceRoomInviteSnapshot,
    now: Instant,
) -> Option<Instant> {
    update_recent_result_with_duration(results, invite, now, Duration::from_secs(10))
}

fn update_recent_result_with_duration(
    results: &mut HashMap<String, RecentOutgoingResult>,
    invite: VoiceRoomInviteSnapshot,
    now: Instant,
    display_duration: Duration,
) -> Option<Instant> {
    let existing = results.get(&invite.invite_id);
    if existing
        .is_some_and(|result| result.invite.resolved_at.is_some() && invite.resolved_at.is_none())
    {
        return existing.map(|result| result.show_until);
    }
    let maximum_deadline = now + display_duration;
    let deadline = existing
        .filter(|result| result.invite.status == invite.status)
        .map(|result| result.show_until.min(maximum_deadline))
        .unwrap_or(maximum_deadline);
    if deadline <= now {
        results.remove(&invite.invite_id);
        return None;
    }
    results.insert(
        invite.invite_id.clone(),
        RecentOutgoingResult {
            invite,
            show_until: deadline,
        },
    );
    Some(deadline)
}

/// Округляет оставшуюся монотонную длительность вверх до миллисекунды.
fn remaining_ms(deadline: Instant, now: Instant) -> u32 {
    let nanos = deadline.saturating_duration_since(now).as_nanos();
    nanos
        .saturating_add(999_999)
        .div_euclid(1_000_000)
        .min(u32::MAX as u128) as u32
}

/// Старый таймер не удаляет результат, срок которого продлён новым событием.
fn remove_due_result(
    results: &mut HashMap<String, RecentOutgoingResult>,
    invite_id: &str,
    now: Instant,
) -> bool {
    if results
        .get(invite_id)
        .is_some_and(|result| result.show_until <= now)
    {
        results.remove(invite_id);
        return true;
    }
    false
}

#[cfg(test)]
mod tests;

//! Локальные монотонные дедлайны приглашений и предварительная просрочка.

use std::time::Duration;

use cheenhub_contracts::realtime::{VoiceRoomInviteSnapshot, VoiceRoomInviteStatus};
use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use web_time::Instant;

use super::{RoomInviteHandle, expire_pending_locally, expiry_delay_ms};

/// Вычисляет оставшееся время с опорой на время сервера при восстановлении.
///
/// Без времени сервера сохраняет полную длительность для совместимости со старыми ответами.
pub(super) fn expiry_delay_for_invite(
    invite: &VoiceRoomInviteSnapshot,
    server_now: Option<DateTime<Utc>>,
) -> u32 {
    let Ok(expires) = DateTime::parse_from_rfc3339(&invite.expires_at) else {
        return 0;
    };
    let origin = server_now.or_else(|| {
        DateTime::parse_from_rfc3339(&invite.created_at)
            .ok()
            .map(|value| value.with_timezone(&Utc))
    });
    origin.map_or(0, |origin| {
        expiry_delay_ms(expires.with_timezone(&Utc), origin)
    })
}

/// Возвращает новый дедлайн, если сервер уточнил ожидание в сторону сокращения.
fn preferred_pending_deadline(
    existing: Option<Instant>,
    candidate: Instant,
    from_server: bool,
) -> Option<Instant> {
    match existing {
        None => Some(candidate),
        Some(previous) if from_server && candidate < previous => Some(candidate),
        _ => None,
    }
}

impl RoomInviteHandle {
    /// Запускает таймер ожидания или сокращает его по более раннему серверному дедлайну.
    pub(super) fn schedule_local_expiry(
        &self,
        invite: &VoiceRoomInviteSnapshot,
        server_now: Option<DateTime<Utc>>,
    ) {
        let invite_id = invite.invite_id.clone();
        let wait_ms = expiry_delay_for_invite(invite, server_now);
        let candidate = Instant::now() + Duration::from_millis(u64::from(wait_ms));
        let existing = self.pending_deadlines.peek().get(&invite_id).copied();
        let Some(deadline) = preferred_pending_deadline(existing, candidate, server_now.is_some())
        else {
            return;
        };
        let mut pending_deadlines = self.pending_deadlines;
        pending_deadlines
            .write()
            .insert(invite_id.clone(), deadline);
        debug!(%invite_id, wait_ms, "started local voice room invitation deadline");
        let handle = self.clone();
        spawn(async move {
            crate::features::runtime::sleep_ms(wait_ms).await;
            if handle.pending_deadlines.peek().get(&invite_id) != Some(&deadline) {
                return;
            }
            let mut pending_deadlines = handle.pending_deadlines;
            pending_deadlines.write().remove(&invite_id);
            let mut invites = handle.invites;
            let mut current = invites.peek().clone();
            let expired_outgoing = current
                .iter()
                .find(|invite| {
                    invite.invite_id == invite_id
                        && invite.caller_user_id == handle.current_user_id
                        && invite.status == VoiceRoomInviteStatus::Pending
                })
                .cloned();
            if expire_pending_locally(&mut current, &invite_id) {
                invites.set(current);
                handle.schedule_terminal_cleanup(&invite_id);
                if let Some(mut outgoing) = expired_outgoing {
                    outgoing.status = VoiceRoomInviteStatus::Expired;
                    handle.remember_result(outgoing);
                }
                debug!(%invite_id, "provisionally expired local voice room invitation");
            }
        });
    }
}

#[cfg(test)]
mod tests;

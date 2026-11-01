//! Локальное состояние и действия для приглашений в серверные голосовые комнаты.

use std::collections::{HashMap, HashSet};

use cheenhub_contracts::realtime::{
    VoiceRoomInviteCandidate, VoiceRoomInviteLifecycleEvent, VoiceRoomInviteResponse,
    VoiceRoomInviteSnapshot, VoiceRoomInviteStatus,
};
use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use web_time::Instant;

use crate::features::realtime::RealtimeHandle;
use crate::features::toast::ToastHandle;

use super::room_invite_realtime;
use super::room_invite_tiles::{
    OutgoingInviteScope, OutgoingInviteTile, RecentOutgoingResult, outgoing_invite_tiles,
};
use super::state::{VoiceConnectionHandle, VoiceConnectionState, VoiceRoomTarget};

/// Состояние приглашений, доступное глобальному toast и панели голосовой комнаты.
#[derive(Clone)]
pub(crate) struct RoomInviteHandle {
    invites: Signal<Vec<VoiceRoomInviteSnapshot>>,
    pending_deadlines: Signal<HashMap<String, Instant>>,
    ignored_ids: Signal<HashSet<String>>,
    candidates: Signal<HashMap<String, VoiceRoomInviteCandidate>>,
    recent_results: Signal<HashMap<String, RecentOutgoingResult>>,
    terminal_cleanup_deadlines: Signal<HashMap<String, Instant>>,
    busy: Signal<bool>,
    realtime: RealtimeHandle,
    voice: VoiceConnectionHandle,
    toasts: ToastHandle,
    current_user_id: String,
}

/// Реактивные сигналы приглашений, передаваемые конструктору одним набором.
pub(super) struct RoomInviteSignals {
    pub(super) invites: Signal<Vec<VoiceRoomInviteSnapshot>>,
    pub(super) pending_deadlines: Signal<HashMap<String, Instant>>,
    pub(super) ignored_ids: Signal<HashSet<String>>,
    pub(super) candidates: Signal<HashMap<String, VoiceRoomInviteCandidate>>,
    pub(super) recent_results: Signal<HashMap<String, RecentOutgoingResult>>,
    pub(super) terminal_cleanup_deadlines: Signal<HashMap<String, Instant>>,
    pub(super) busy: Signal<bool>,
}

impl RoomInviteHandle {
    /// Создаёт общий контекст без сетевых запросов и побочных эффектов.
    pub(super) fn new(
        signals: RoomInviteSignals,
        realtime: RealtimeHandle,
        voice: VoiceConnectionHandle,
        toasts: ToastHandle,
        current_user_id: String,
    ) -> Self {
        Self {
            invites: signals.invites,
            pending_deadlines: signals.pending_deadlines,
            ignored_ids: signals.ignored_ids,
            candidates: signals.candidates,
            recent_results: signals.recent_results,
            terminal_cleanup_deadlines: signals.terminal_cleanup_deadlines,
            busy: signals.busy,
            realtime,
            voice,
            toasts,
            current_user_id,
        }
    }

    /// Возвращает первое ожидающее приглашение для этого пользователя и устройства.
    pub(crate) fn incoming(&self) -> Option<VoiceRoomInviteSnapshot> {
        incoming_invite(
            &(self.invites)(),
            &(self.ignored_ids)(),
            &self.current_user_id,
            &(self.pending_deadlines)(),
            Instant::now(),
        )
    }

    /// Показывает, выполняется ли отправка или ответ на приглашение.
    pub(crate) fn busy(&self) -> bool {
        (self.busy)()
    }

    /// Возвращает отправленные приглашения для заданной комнаты.
    pub(crate) fn outgoing_for_room(
        &self,
        server_id: &str,
        room_id: &str,
    ) -> Vec<VoiceRoomInviteSnapshot> {
        (self.invites)()
            .into_iter()
            .filter(|invite| {
                invite.caller_user_id == self.current_user_id
                    && invite.server_id == server_id
                    && invite.room_id == room_id
            })
            .collect()
    }

    /// Запоминает никнеймы и аватары пользователей из списка выбора приглашённых.
    pub(crate) fn remember_candidates(&self, loaded: &[VoiceRoomInviteCandidate]) {
        let mut candidates = self.candidates.peek().clone();
        for candidate in loaded {
            candidates.insert(candidate.user_id.clone(), candidate.clone());
        }
        let mut signal = self.candidates;
        signal.set(candidates);
    }

    /// Возвращает отдельные плитки дозвона без подмены настоящих голосовых участников.
    pub(crate) fn outgoing_tiles_for_room(
        &self,
        server_id: &str,
        room_id: &str,
        present_users: &HashSet<String>,
    ) -> Vec<OutgoingInviteTile> {
        outgoing_invite_tiles(
            &(self.invites)(),
            &(self.pending_deadlines)(),
            &(self.recent_results)(),
            &(self.candidates)(),
            OutgoingInviteScope {
                caller_id: &self.current_user_id,
                server_id,
                room_id,
            },
            present_users,
            Instant::now(),
        )
    }

    /// Обновляет приглашение после адресного серверного события.
    pub(crate) fn apply_event(&self, event: VoiceRoomInviteLifecycleEvent) {
        let invite = event.invite;
        if invite.caller_user_id != self.current_user_id
            && invite.recipient_user_id != self.current_user_id
        {
            return;
        }
        let mut invites = self.invites.peek().clone();
        let previous = invites
            .iter()
            .find(|existing| existing.invite_id == invite.invite_id)
            .cloned();
        let applied = upsert_invite(&mut invites, invite.clone());
        let still_pending = invites.iter().any(|current| {
            current.invite_id == invite.invite_id
                && current.status == VoiceRoomInviteStatus::Pending
        });
        let mut signal = self.invites;
        signal.set(invites);
        if still_pending {
            self.schedule_local_expiry(&invite, None);
        } else {
            let mut pending_deadlines = self.pending_deadlines;
            pending_deadlines.write().remove(&invite.invite_id);
        }
        if applied && !still_pending {
            self.schedule_terminal_cleanup(&invite.invite_id);
        }
        if applied
            && invite.caller_user_id == self.current_user_id
            && invite.status != VoiceRoomInviteStatus::Pending
            && results::should_remember_terminal_event(
                previous.as_ref(),
                &self.recent_results.peek(),
                &invite.invite_id,
                Instant::now(),
            )
        {
            self.remember_result(invite);
        }
    }

    /// Восстанавливает состояние, удаляя отсутствовавшие на сервере старые ожидания.
    ///
    /// События, пришедшие после начала запроса, не удаляются, а завершённые
    /// приглашения не возвращаются в `Pending` из-за запоздалого снимка.
    pub(crate) fn apply_recovered(
        &self,
        recovered: Vec<VoiceRoomInviteSnapshot>,
        pending_at_request_start: &HashSet<String>,
        server_now: Option<&str>,
    ) {
        let server_now = server_now
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc));
        let resolved = results::recovered_outgoing_resolutions(&recovered, &self.current_user_id);
        let mut invites = self.invites.peek().clone();
        let before = invites.len();
        reconcile_recovered_invites(
            &mut invites,
            recovered,
            pending_at_request_start,
            &self.current_user_id,
        );
        debug!(
            removed = before.saturating_sub(invites.len()),
            current = invites.len(),
            "reconciled recovered voice room invitations"
        );
        let terminals = invites
            .iter()
            .filter(|invite| invite.status != VoiceRoomInviteStatus::Pending)
            .map(|invite| invite.invite_id.clone())
            .collect::<Vec<_>>();
        let expiries = invites
            .iter()
            .filter(|invite| invite.status == VoiceRoomInviteStatus::Pending)
            .cloned()
            .collect::<Vec<_>>();
        let pending_ids = expiries
            .iter()
            .map(|invite| invite.invite_id.as_str())
            .collect::<HashSet<_>>();
        let mut pending_deadlines = self.pending_deadlines;
        pending_deadlines
            .write()
            .retain(|invite_id, _| pending_ids.contains(invite_id.as_str()));
        let mut signal = self.invites;
        signal.set(invites);
        for invite in &expiries {
            self.schedule_local_expiry(invite, server_now);
        }
        for invite_id in terminals {
            self.schedule_terminal_cleanup(&invite_id);
        }
        for invite in resolved {
            self.remember_recovered_result(invite, server_now);
        }
    }

    /// Запрашивает восстановление при подключении realtime.
    pub(crate) fn recover(&self) {
        let pending_at_request_start: HashSet<String> = self
            .invites
            .peek()
            .iter()
            .filter(|invite| invite.status == VoiceRoomInviteStatus::Pending)
            .map(|invite| invite.invite_id.clone())
            .collect();
        let realtime = self.realtime.clone();
        let handle = self.clone();
        spawn(async move {
            match room_invite_realtime::list(&realtime).await {
                Ok(snapshot) => {
                    handle.apply_recovered(
                        snapshot.invites,
                        &pending_at_request_start,
                        snapshot.server_now.as_deref(),
                    );
                }
                Err(error) => warn!(%error, "failed to recover voice room invitations"),
            }
        });
    }

    /// Отправляет приглашение выбранному участнику сервера.
    pub(crate) fn send(&self, server_id: String, room_id: String, recipient_id: String) {
        if self.busy() {
            return;
        }
        let realtime = self.realtime.clone();
        let handle = self.clone();
        let mut busy = self.busy;
        busy.set(true);
        spawn(async move {
            match room_invite_realtime::send(&realtime, server_id, room_id, recipient_id).await {
                Ok(event) => handle.apply_event(event),
                Err(error) => handle
                    .toasts
                    .error(format!("Не удалось пригласить: {error}")),
            }
            busy.set(false);
        });
    }

    /// Принимает приглашение и подключает медиа только на этом устройстве.
    pub(crate) fn accept(&self, invite_id: &str) {
        self.respond(invite_id, VoiceRoomInviteResponse::Accept);
    }

    /// Отклоняет приглашение с синхронизацией на сервере.
    pub(crate) fn decline(&self, invite_id: &str) {
        self.respond(invite_id, VoiceRoomInviteResponse::Decline);
    }

    fn respond(&self, invite_id: &str, response: VoiceRoomInviteResponse) {
        if self.busy() {
            return;
        }
        let Some(invite) = self
            .invites
            .peek()
            .iter()
            .find(|invite| {
                invite.invite_id == invite_id
                    && invite.recipient_user_id == self.current_user_id
                    && invite.status == VoiceRoomInviteStatus::Pending
            })
            .cloned()
        else {
            return;
        };
        let realtime = self.realtime.clone();
        let handle = self.clone();
        let mut busy = self.busy;
        busy.set(true);
        spawn(async move {
            match room_invite_realtime::respond(&realtime, invite.invite_id.clone(), response).await
            {
                Ok(event) => {
                    handle.apply_event(event);
                    if response == VoiceRoomInviteResponse::Accept {
                        let target = VoiceRoomTarget::server(
                            invite.server_id,
                            invite.room_id,
                            invite.room_name,
                        );
                        if should_join_after_accept(&handle.voice.state(), &target) {
                            info!(
                                invite_id = %invite.invite_id,
                                server_id = %target.server_id,
                                room_id = %target.room_id,
                                "joining voice room after accepting invitation"
                            );
                            handle.voice.join(target);
                        }
                    }
                }
                Err(error) => handle.toasts.error(format!("Не удалось ответить: {error}")),
            }
            busy.set(false);
        });
    }

    /// Скрывает только на текущем устройстве, не изменяя серверный статус.
    pub(crate) fn ignore(&self, invite_id: &str) {
        let mut ignored = self.ignored_ids.peek().clone();
        ignored.insert(invite_id.to_owned());
        let mut signal = self.ignored_ids;
        signal.set(ignored);
    }

    /// Отменяет ещё не принятое приглашение от имени отправителя.
    pub(crate) fn cancel(&self, invite_id: String) {
        if self.busy() {
            return;
        }
        let realtime = self.realtime.clone();
        let handle = self.clone();
        let mut busy = self.busy;
        busy.set(true);
        spawn(async move {
            match room_invite_realtime::cancel(&realtime, invite_id).await {
                Ok(event) => handle.apply_event(event),
                Err(error) => handle.toasts.error(format!("Не удалось отменить: {error}")),
            }
            busy.set(false);
        });
    }
}

/// После принятия приглашения повторяет вход, если соединение ещё не подтверждено.
fn should_join_after_accept(state: &VoiceConnectionState, target: &VoiceRoomTarget) -> bool {
    !state
        .connected_target()
        .is_some_and(|connected| connected.matches(target))
}

/// Округляет задержку вверх: таймер не должен проснуться раньше серверного дедлайна.
fn expiry_delay_ms(deadline: DateTime<Utc>, now: DateTime<Utc>) -> u32 {
    let remaining = deadline - now;
    if remaining <= chrono::Duration::zero() {
        return 0;
    }
    let whole_ms = remaining.num_milliseconds();
    let rounded_ms = if remaining > chrono::Duration::milliseconds(whole_ms) {
        whole_ms.saturating_add(1)
    } else {
        whole_ms
    };
    rounded_ms.clamp(1, u32::MAX as i64) as u32
}

/// Сохраняет не более одной записи на ID приглашения.
///
/// Возвращает `true` только если событие действительно изменило локальное состояние.
fn upsert_invite(
    invites: &mut Vec<VoiceRoomInviteSnapshot>,
    invite: VoiceRoomInviteSnapshot,
) -> bool {
    if let Some(existing) = invites
        .iter_mut()
        .find(|current| current.invite_id == invite.invite_id)
    {
        // Локальный Expired предварителен: Pending его не отменяет, серверный итог уточняет.
        let provisional_expiry =
            existing.status == VoiceRoomInviteStatus::Expired && existing.resolved_at.is_none();
        let authoritative_resolution =
            invite.status != VoiceRoomInviteStatus::Pending && invite.resolved_at.is_some();
        if existing.status == VoiceRoomInviteStatus::Pending
            || (provisional_expiry && authoritative_resolution)
        {
            let changed = *existing != invite;
            *existing = invite;
            return changed;
        }
        false
    } else {
        invites.push(invite);
        true
    }
}

/// Согласует серверный снимок с событиями, пришедшими во время восстановления.
fn reconcile_recovered_invites(
    invites: &mut Vec<VoiceRoomInviteSnapshot>,
    recovered: Vec<VoiceRoomInviteSnapshot>,
    pending_at_request_start: &HashSet<String>,
    current_user_id: &str,
) {
    let authoritative_ids: HashSet<String> = recovered
        .iter()
        .filter(|invite| {
            invite.caller_user_id == current_user_id || invite.recipient_user_id == current_user_id
        })
        .map(|invite| invite.invite_id.clone())
        .collect();
    invites.retain(|invite| {
        invite.status != VoiceRoomInviteStatus::Pending
            || !pending_at_request_start.contains(&invite.invite_id)
            || authoritative_ids.contains(&invite.invite_id)
    });
    for invite in recovered {
        if invite.caller_user_id == current_user_id || invite.recipient_user_id == current_user_id {
            upsert_invite(invites, invite);
        }
    }
}

/// Помечает локальную просрочку предварительно, не подменяя окончательного решения сервера.
fn expire_pending_locally(invites: &mut [VoiceRoomInviteSnapshot], invite_id: &str) -> bool {
    let Some(invite) = invites.iter_mut().find(|invite| {
        invite.invite_id == invite_id && invite.status == VoiceRoomInviteStatus::Pending
    }) else {
        return false;
    };
    invite.status = VoiceRoomInviteStatus::Expired;
    invite.resolved_at = None;
    true
}

/// Выбирает ожидающее входящее приглашение, не скрытое на данном устройстве.
fn incoming_invite(
    invites: &[VoiceRoomInviteSnapshot],
    ignored_ids: &HashSet<String>,
    current_user_id: &str,
    pending_deadlines: &HashMap<String, Instant>,
    now: Instant,
) -> Option<VoiceRoomInviteSnapshot> {
    invites
        .iter()
        .filter(|invite| {
            invite.recipient_user_id == current_user_id
                && invite.status == VoiceRoomInviteStatus::Pending
                && !ignored_ids.contains(&invite.invite_id)
                && pending_deadlines
                    .get(&invite.invite_id)
                    .is_some_and(|deadline| *deadline > now)
        })
        .min_by(|left, right| left.created_at.cmp(&right.created_at))
        .cloned()
}

mod cleanup;
mod expiry;
mod results;

#[cfg(test)]
mod tests;

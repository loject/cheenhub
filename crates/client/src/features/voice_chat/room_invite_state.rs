//! Локальное состояние и действия для приглашений в серверные голосовые комнаты.

use std::collections::HashSet;

use cheenhub_contracts::realtime::{
    VoiceRoomInviteLifecycleEvent, VoiceRoomInviteResponse, VoiceRoomInviteSnapshot,
    VoiceRoomInviteStatus,
};
use dioxus::prelude::*;

use crate::features::realtime::RealtimeHandle;
use crate::features::toast::ToastHandle;

use super::room_invite_realtime;
use super::state::{VoiceConnectionHandle, VoiceRoomTarget};

/// Состояние приглашений, доступное глобальному toast и панели голосовой комнаты.
#[derive(Clone)]
pub(crate) struct RoomInviteHandle {
    invites: Signal<Vec<VoiceRoomInviteSnapshot>>,
    ignored_ids: Signal<HashSet<String>>,
    busy: Signal<bool>,
    realtime: RealtimeHandle,
    voice: VoiceConnectionHandle,
    toasts: ToastHandle,
    current_user_id: String,
}

impl RoomInviteHandle {
    /// Создаёт общий контекст без сетевых запросов и побочных эффектов.
    pub(super) fn new(
        invites: Signal<Vec<VoiceRoomInviteSnapshot>>,
        ignored_ids: Signal<HashSet<String>>,
        busy: Signal<bool>,
        realtime: RealtimeHandle,
        voice: VoiceConnectionHandle,
        toasts: ToastHandle,
        current_user_id: String,
    ) -> Self {
        Self {
            invites,
            ignored_ids,
            busy,
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

    /// Обновляет приглашение после адресного серверного события.
    pub(crate) fn apply_event(&self, event: VoiceRoomInviteLifecycleEvent) {
        let invite = event.invite;
        if invite.caller_user_id != self.current_user_id
            && invite.recipient_user_id != self.current_user_id
        {
            return;
        }
        let mut invites = self.invites.peek().clone();
        upsert_invite(&mut invites, invite);
        let mut signal = self.invites;
        signal.set(invites);
    }

    /// Восстанавливает состояние; более свежее локальное завершение не заменяет ожидающим.
    pub(crate) fn apply_recovered(&self, recovered: Vec<VoiceRoomInviteSnapshot>) {
        let mut invites = self.invites.peek().clone();
        for invite in recovered {
            if invite.caller_user_id != self.current_user_id
                && invite.recipient_user_id != self.current_user_id
            {
                continue;
            }
            let existing_terminal = invites.iter().any(|current| {
                current.invite_id == invite.invite_id
                    && current.status != VoiceRoomInviteStatus::Pending
            });
            if !existing_terminal {
                upsert_invite(&mut invites, invite);
            }
        }
        let mut signal = self.invites;
        signal.set(invites);
    }

    /// Запрашивает восстановление при подключении realtime.
    pub(crate) fn recover(&self) {
        let realtime = self.realtime.clone();
        let handle = self.clone();
        spawn(async move {
            match room_invite_realtime::list(&realtime).await {
                Ok(snapshot) => handle.apply_recovered(snapshot.invites),
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
                Ok(event) => {
                    handle.apply_event(event);
                    handle.toasts.success("Приглашение отправлено.");
                }
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
                        if !handle
                            .voice
                            .state()
                            .active_target()
                            .is_some_and(|active| active.matches(&target))
                        {
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

/// Сохраняет не более одной записи на ID приглашения.
fn upsert_invite(invites: &mut Vec<VoiceRoomInviteSnapshot>, invite: VoiceRoomInviteSnapshot) {
    if let Some(existing) = invites
        .iter_mut()
        .find(|current| current.invite_id == invite.invite_id)
    {
        // Терминальные состояния неизменяемы: запоздалый ACK отправки не возвращает Pending.
        if existing.status == VoiceRoomInviteStatus::Pending {
            *existing = invite;
        }
    } else {
        invites.push(invite);
    }
}

/// Выбирает ожидающее входящее приглашение, не скрытое на данном устройстве.
fn incoming_invite(
    invites: &[VoiceRoomInviteSnapshot],
    ignored_ids: &HashSet<String>,
    current_user_id: &str,
) -> Option<VoiceRoomInviteSnapshot> {
    invites
        .iter()
        .filter(|invite| {
            invite.recipient_user_id == current_user_id
                && invite.status == VoiceRoomInviteStatus::Pending
                && !ignored_ids.contains(&invite.invite_id)
        })
        .min_by(|left, right| left.created_at.cmp(&right.created_at))
        .cloned()
}

#[cfg(test)]
mod tests;
